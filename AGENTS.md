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
- Este é o modo que está sendo desenvolvido agora

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

```json
{
  "fields": [
    { "key": "name",     "type": "string",  "required": true  },
    { "key": "amount",   "type": "number",  "required": true  },
    { "key": "currency", "type": "enum",    "required": false,
      "values": ["BRL", "USD"], "default": "BRL" },
    { "key": "items",    "type": "array",   "required": true,
      "item": { "type": "object", "fields": [
        { "key": "label",  "type": "string", "required": true },
        { "key": "value",  "type": "number", "required": true }
      ]}}
  ]
}
```

Tipos suportados: `string`, `number`, `boolean`, `enum`, `array`, `object`

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
| `TemplateName` | identificador de um template. ex: `"invoice"` |
| `TemplateSource` | arquivos locais antes de qualquer processamento |
| `Schema` | contrato de dados parseado do `schema.json` |
| `ResolvedTemplate` | template pronto para compilar: arquivos + schema |
| `RenderRequest` | nome do template + payload JSON |
| `Document` | bytes do PDF resultante |
| `RenderContext` | mapa de arquivos montado para o compilador Typst |
| `ValidationError` | erro de validação do payload contra o schema |
| `CompilationError` | erro do compilador Typst |

---

## Arquitetura — trait central

O servidor HTTP nunca acessa o filesystem diretamente. Toda resolução de template passa pelo trait `TemplateResolver`:

```rust
#[async_trait]
pub trait TemplateResolver: Send + Sync {
    async fn resolve(&self, name: &TemplateName) -> Result<ResolvedTemplate, ResolverError>;
    async fn list(&self) -> Result<Vec<TemplateName>, ResolverError>;
}
```

Implementações:
- `FilesystemResolver` — lê do filesystem (Modo 1, implementar agora)
- `StoreResolver` — lê da Store (Modo 2, NÃO implementar agora)

O `RenderService` recebe um `Arc<dyn TemplateResolver>`. Os handlers HTTP recebem o `RenderService`. Nada no servidor HTTP conhece filesystem ou store diretamente.

---

## Estrutura do workspace Cargo

```
papelon/
  Cargo.toml              ← workspace root
  CLAUDE.md               ← este arquivo
  DECISIONS.md            ← log de decisões arquiteturais
  crates/
    papelon-core/         ← domínio + tipos + traits (sem HTTP, sem filesystem)
      src/
        lib.rs
        domain/           ← tipos do domínio
        resolver/         ← trait TemplateResolver
        schema/           ← parser e validação do schema.json
        compiler/         ← integração com Typst
        render/           ← RenderService
    papelon-server/       ← servidor HTTP (axum)
      src/
        main.rs
        routes/
        errors/
    papelon-cli/          ← CLI (clap) — só `papelon init` no MVP
      src/
        main.rs
```

**Regra:** `papelon-core` não tem dependências de HTTP, filesystem, ou IO. É testável em isolamento com `MemoryResolver` nos testes.

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

- Erros de domínio são sempre tipados com `thiserror` — sem `unwrap()` fora de testes
- Handlers HTTP nunca retornam erros genéricos — sempre mapeiam para o formato JSON acima
- Testes de integração usam `MemoryResolver` — sem dependência de filesystem nos testes de core
- `_shared/` é incluído no contexto de compilação de qualquer template automaticamente
- Subpastas dentro de um template são permitidas — o caminho relativo é preservado no `RenderContext`
- O `schema.json` é validado no momento da leitura do template (startup ou watch), não só no render

---

## O que NÃO fazer neste MVP

- Não implementar Store (blobs, manifests, refs, tags)
- Não implementar protocolo de push CLI→servidor
- Não implementar autenticação (token, OAuth)
- Não implementar SDKs para outras linguagens
- Não implementar cache de compilação
- Não implementar `papelon gc`
- Não implementar S3 backend

Se surgir a necessidade de algum desses durante o desenvolvimento, registrar em `DECISIONS.md` e discutir antes de implementar.