# Contexto do Projeto

Este arquivo é lido automaticamente pelo Claude Code antes de qualquer sessão.
Contém todo o contexto de design, decisões arquiteturais e regras do projeto.

---

## O que é este projeto

Um servidor HTTP em Rust para geração de PDFs server-side usando **Typst** como engine de compilação. Desenvolvido para ser uma alternativa leve, performática e language-agnostic às soluções atuais (Puppeteer, pdfmake, ReportLab, etc).

O diferencial central: qualquer sistema em qualquer linguagem integra via HTTP simples. Não é uma biblioteca — é um serviço.

---

## Dois modos de operação

### Modo 1 — Filesystem (MVP atual)

```bash
papelon serve --templates ./templates 8080
```

- Lê templates diretamente da pasta `./templates`
- Templates ficam dentro do container — atualizar template = rebuild + redeploy
- **Sem store, sem tags, sem versionamento**
- Rotas: `POST /render/:template` (sem tag)
- **Este é o único modo sendo desenvolvido agora**

### Modo 2 — Store (versão futura, NÃO implementar agora)

```bash
papelon serve --store /path/to/store 8080
```

- Templates publicados via `papelon push`
- Versionamento com tags (`invoice:latest`, `invoice:v2.1`)
- Rotas: `POST /render/:template/:tag`
- Backends plugáveis: filesystem, S3, GCS

**Não implementar o Modo 2 ainda. O código deve estar preparado para ele via trait, mas sem implementação.**

---

## Estrutura de templates (o que o usuário cria)

```
templates/
  invoice/
    main.typ        ← template Typst (obrigatório)
    schema.json     ← contrato de dados (obrigatório)
    assets/         ← opcional
      logo.svg
  welcome/
    main.typ
    schema.json
  _shared/          ← arquivos disponíveis para todos os templates
    brand.typ
    logo.svg
```

### `schema.json` — formato
A especificação oficial do schema encontra-se em:

- docs/schema-v1.md

Sempre consulte este documento antes de implementar qualquer funcionalidade relacionada à validação.

As implementações devem seguir rigorosamente a especificação.
Não introduza novos campos ou altere o comportamento da DSL sem atualizar primeiro a especificação.

### Como o Typst recebe os dados

```typst
#let data = json(bytes(sys.inputs.data))
```

O servidor injeta o payload JSON via `sys.inputs.data` na compilação.

---

## Domínio — termos canônicos

Usar **exatamente** estes nomes no código (structs, variáveis, funções, comentários):

| Termo | O que é |
|---|---|
| `TemplateName` | newtype sobre String. Identificador de um template. Invariante: não vazio, somente ASCII alfanumérico, hífen (`-`) e underscore (`_`). |
| `Schema` | Estrutura recursiva parseada do `schema.json`. Contém as regras de validação do payload. |
| `TemplateSource` | Mapa de `caminho relativo → bytes`. Arquivos brutos do template lidos pelo loader. |
| `TemplateBundle` | `TemplateSource` + `Schema`. Tudo que o compilador precisa — autocontido. Retorno do `TemplateLoader`. |
| `RenderRequest` | `TemplateName` + `serde_json::Value` (payload). Input do `RenderService`. |
| `Document` | `Vec<u8>` (bytes do PDF) + `TemplateName` que o gerou. Output do `RenderService`. |
| `SchemaVersionError` | Erro de carregamento: `schemaVersion` ausente ou não suportado. |
| `SchemaError` | Erro de carregamento: schema estruturalmente inválido (properties sem object, enum com tipos mistos, etc). |
| `ValidationError` | Erro de render: payload não satisfaz o schema (campo obrigatório ausente, tipo errado, enum inválido). |
| `CompilationError` | Erro de render: compilador Typst falhou. Contém mensagem e linha do erro. |
| `LoaderError` | Enum que unifica erros do `TemplateLoader`: `TemplateNotFound`, `SchemaVersionError`, `SchemaError`, `IoError`. |
| `RenderError` | Enum que unifica erros do `RenderService`: `TemplateNotFound`, `ValidationError`, `CompilationError`. |

### Distinção crítica: erro de carregamento vs erro de render

- **Erros de carregamento** (`SchemaVersionError`, `SchemaError`) são detectados quando o template é lido do disco — no startup ou no `--watch`. Nunca chegam ao render.
- **Erros de render** (`ValidationError`, `CompilationError`) são detectados em runtime, por request.

---

## Arquitetura — trait central

O servidor HTTP nunca acessa o filesystem diretamente. Toda leitura de template passa pelo trait `TemplateLoader`:

```rust
#[async_trait]
pub trait TemplateLoader: Send + Sync {
    async fn load(&self, name: &TemplateName) -> Result<TemplateBundle, LoaderError>;
    async fn list(&self) -> Result<Vec<TemplateName>, LoaderError>;
}
```

Implementações:
- `FilesystemLoader` — lê do filesystem (Modo 1, implementar agora)
- `StoreLoader` — lê da Store (Modo 2, **NÃO implementar agora**)

O `RenderService` recebe um `Arc<dyn TemplateLoader>`. Os handlers HTTP recebem o `RenderService`. Nada no servidor HTTP conhece filesystem ou store diretamente.

---

## Arquitetura do compilador Typst — Anti-Corruption Layer

A crate `typst` tem API instável entre versões minor. Para proteger o resto da aplicação dessas mudanças, o módulo `compiler/` é um **Anti-Corruption Layer (ACL)**: nenhum outro módulo importa `typst::*` diretamente.

### Regra absoluta

> Se qualquer arquivo fora de `crates/papelon-core/src/compiler/` contiver `use typst::` ou `use typst_pdf::`, a arquitetura foi violada.

O compilador Rust reforça isso: `world.rs` e `fonts.rs` são `pub(super)` — invisíveis fora do módulo.

### Interface pública — estável, nunca muda

Definida no módulo `compiler`. É tudo que o `RenderService` e os testes conhecem:

```rust
// O único erro de compilação que o mundo externo conhece
// Não contém nenhum tipo de typst::*
#[derive(Debug, thiserror::Error)]
pub enum CompilationError {
    #[error("compilation failed: {message}")]
    Failed { message: String, line: Option<u32> },

    #[error("pdf export failed: {0}")]
    ExportFailed(String),
}

// O único trait que o RenderService usa
// Nunca expõe typst::* na assinatura
#[async_trait::async_trait]
pub trait PdfCompiler: Send + Sync {
    async fn compile(
        &self,
        source: &TemplateSource,
        payload: &serde_json::Value,
    ) -> Result<Document, CompilationError>;
}

// A única struct concreta exportada
pub struct TypstCompiler { /* internals privados */ }

impl TypstCompiler {
    pub fn new() -> Self { ... }
}
```

### Arquivos internos — podem mudar com o Typst

Todos `pub(super)` — invisíveis fora de `compiler/`:

```
compiler/
  compiler.rs  ← interface pública: PdfCompiler, CompilationError, TypstCompiler
  world.rs     ← implementa typst::World. ÚNICO arquivo que importa typst::*
  fonts.rs     ← FontRegistry: carrega typst-assets + fontes do template
```

**`world.rs`** contém:
- `struct TemplateWorld` que implementa `typst::World`
- `fn compile(source, payload, fonts) -> Result<Vec<u8>, CompilationError>` — função privada que orquestra a compilação e converte erros do Typst para `CompilationError`
- `fn map_errors(Vec<SourceDiagnostic>) -> CompilationError` — único lugar que conhece `SourceDiagnostic`
- `fn build_inputs(payload: &serde_json::Value) -> typst::foundations::Dict` — converte o payload para o formato do Typst

**`fonts.rs`** contém:
- `struct FontRegistry` — carregada uma vez no startup, compartilhada entre compilações via `Arc`
- Carrega fontes padrão de `typst-assets`
- Carrega fontes customizadas do template (arquivos `.ttf`, `.otf` em `assets/`)

### Quando o Typst mudar a API

O impacto fica **100% dentro de `world.rs` e `fonts.rs`**. Exemplo de mudanças históricas e onde seriam absorvidas:

| Mudança do Typst | Onde absorver |
|---|---|
| `PdfOptions` ganhou novos campos | `world.rs` — só `fn compile()` |
| `compile()` mudou o tipo de retorno | `world.rs` — só `fn compile()` |
| `SourceDiagnostic` ganhou/perdeu campos | `world.rs` — só `fn map_errors()` |
| `FileId`/`VirtualPath` mudaram de módulo | `world.rs` — imports no topo |
| `FontBook` mudou de API | `fonts.rs` — só `FontRegistry` |

### MockCompiler para testes

O `RenderService` e os handlers HTTP são testados com `MockCompiler` — sem Typst, sem compilação real:

```rust
// Em testes — nunca em produção
struct MockCompiler;

#[async_trait]
impl PdfCompiler for MockCompiler {
    async fn compile(&self, _: &TemplateSource, _: &serde_json::Value)
        -> Result<Document, CompilationError>
    {
        // %PDF é o magic byte — válido o suficiente para testes de integração HTTP
        Ok(Document::new(b"%PDF-1.4 mock".to_vec()))
    }
}
```

Testes reais de compilação Typst ficam em `compiler/world.rs` como testes de módulo (`#[cfg(test)]`) — isolados, mais lentos, e não bloqueiam os testes de domínio.

---

## Estrutura do workspace Cargo

```
papelon/
  Cargo.toml              ← workspace root
  CLAUDE.md               ← este arquivo
  DECISIONS.md            ← log de decisões arquiteturais
  ROADMAP.md              ← tarefas por fase com checkboxes
  docs/
    schema-v1.md          ← referência schema.json
    typst-integration.md  ← referência da API da crate typst
  crates/
    papelon-core/         ← domínio + tipos + traits (sem HTTP, sem filesystem direto)
      src/
        lib.rs
        template_name.rs  ← TemplateName
        template.rs       ← TemplateSource, TemplateBundle, Document
        render_request.rs ← RenderRequest
        errors.rs         ← ValidationError, CompilationError
        loader.rs         ← trait TemplateLoader + LoaderError
        loader/           ← MemoryLoader (testes)
        schema.rs         ← parser, validador, Schema, SchemaField, erros de schema
        compiler.rs       ← ACL do Typst: interface pública
        compiler/         ← world.rs e fonts.rs privados
        render/           ← RenderService, RenderError
    papelon-server/       ← servidor HTTP (axum)
      src/
        main.rs
        routes/
        errors/
    papelon-cli/          ← CLI (clap) — só `papelon init` no MVP
      src/
        main.rs
```

**Regra:** `papelon-core` não tem dependências de HTTP. É testável em isolamento com `MemoryLoader` nos testes.

**Nota:** o módulo dentro de `papelon-core/src/` para o trait e implementações é `loader`, não `resolver` — acompanha a renomeação do trait.

---

## API HTTP — Modo 1

### `POST /render/:template`

Request:
```
Content-Type: application/json
Body: { payload conforme schema.json do template }
```

Response (sucesso):
```
200 OK
Content-Type: application/pdf
Body: bytes do PDF
```

Response (erros):
```
404 — template não encontrado
422 — payload inválido (com lista de violations)
500 — erro de compilação Typst
```

Formato de erro:
```json
{
  "error": "schema_validation_failed",
  "message": "Payload validation failed",
  "violations": [
    { "field": "amount", "issue": "required field missing" },
    { "field": "currency", "issue": "invalid enum value 'EUR'" }
  ]
}
```

### `GET /templates`

Response:
```json
{
  "templates": [
    { "name": "invoice" },
    { "name": "welcome" }
  ]
}
```

### `GET /healthz`

```json
{ "status": "ok" }
```

---

## Crates utilizadas

| Crate | Uso |
|---|---|
| `axum` | servidor HTTP |
| `tokio` | runtime async |
| `serde` + `serde_json` | serialização |
| `typst` + `typst-pdf` | compilador de templates |
| `notify` | file watching (modo --watch) |
| `tracing` + `tracing-subscriber` | logs estruturados |
| `thiserror` | erros de domínio tipados |
| `anyhow` | erros na CLI |
| `clap` | CLI |
| `bytes` | manipulação de bytes |
| `async-trait` | trait async |

---

## Regras de código

- **Nenhum arquivo fora de `compiler/` importa `typst::*` ou `typst_pdf::*`** — violação do ACL
- Erros de domínio são sempre tipados com `thiserror` — sem `unwrap()` fora de testes
- Handlers HTTP nunca retornam erros genéricos — sempre mapeiam para o formato JSON acima
- Testes de integração usam `MemoryLoader` — sem dependência de filesystem nos testes de core
- `_shared/` é incluído no contexto de compilação de qualquer template automaticamente
- Subpastas dentro de um template são permitidas — o caminho relativo é preservado no `TemplateSource`
- Erros de schema são detectados no carregamento do template, nunca no render
- Campos desconhecidos na raiz do `schema.json` são ignorados com warning — não rejeitados
- Usar organização de módulos Rust moderna (edição 2018+): preferir `foo.rs` + `foo/` para submódulos e não criar novos arquivos `mod.rs`. Ao tocar módulos existentes, migrar gradualmente para esse padrão quando estiver no escopo da mudança.
- Não criar `pub use` apenas como alias/conveniência de importação entre módulos. Prefira expor o submódulo real (`pub mod memory;`, `pub mod filesystem;`) e importar pelo caminho explícito.

---

## O que NÃO fazer neste MVP

- Não implementar Store (blobs, manifests, refs, tags)
- Não implementar `StoreLoader`
- Não implementar protocolo de push CLI→servidor
- Não implementar autenticação (token, OAuth)
- Não implementar SDKs para outras linguagens
- Não implementar cache de compilação
- Não implementar `papelon gc`
- Não implementar S3 backend
- Não suportar pacotes do Typst Universe (`@preview/...`) — retornar erro claro

Se surgir a necessidade de algum desses durante o desenvolvimento, registrar em `DECISIONS.md` e discutir antes de implementar.
