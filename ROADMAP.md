# ROADMAP.md — MVP Fase 1 (Modo Filesystem)

Status atual do desenvolvimento. Atualizar conforme tarefas são concluídas.

---

## Objetivo do MVP

Validar tecnicamente a ideia central: **Typst como engine de PDF server-side, acessível via HTTP, language-agnostic**.

Critério de sucesso: `docker run` sobe o servidor, `curl POST /render/invoice` com JSON retorna um PDF válido em menos de 1 segundo.

---

## Fase 1 — Fundação (sem. 1–2)

**Objetivo:** workspace funcionando, tipos do domínio definidos, leitura de templates do filesystem.

- [ ] Criar workspace Cargo com três crates: `papelon-core`, `papelon-server`, `papelon-cli`
- [ ] Definir tipos do domínio em `papelon-core`:
  - [ ] `TemplateName` (newtype sobre String com validação)
  - [ ] `Schema` e `SchemaField` (parseados do schema.json)
  - [ ] `TemplateSource` (mapa de caminho → bytes)
  - [ ] `ResolvedTemplate` (TemplateSource + Schema)
  - [ ] `RenderRequest` (TemplateName + serde_json::Value)
  - [ ] `Document` (Bytes do PDF)
  - [ ] `ValidationError` e `CompilationError` (com thiserror)
- [ ] Definir trait `TemplateResolver` em `papelon-core`
- [ ] Implementar `FilesystemResolver`:
  - [ ] Escanear pasta de templates recursivamente
  - [ ] Parsear `schema.json` de cada template
  - [ ] Incluir arquivos do `_shared/` no contexto de cada template
  - [ ] Preservar estrutura de subpastas no mapa de arquivos
- [ ] Implementar `MemoryResolver` para testes (recebe mapa em memória)
- [ ] Parser e validador do `schema.json`:
  - [ ] Tipos: string, number, boolean, enum, array, object
  - [ ] Campos obrigatórios vs opcionais
  - [ ] Defaults para campos opcionais
  - [ ] `validate(schema, payload) → Result<(), Vec<ValidationError>>`
- [ ] Testes unitários: FilesystemResolver lê templates corretamente, validação de schema
- [ ] CI: GitHub Actions com `cargo fmt`, `cargo clippy`, `cargo test`

**Critério de saída:** consigo criar um `FilesystemResolver` apontando para uma pasta com templates, chamar `resolve("invoice")` e receber os arquivos e o schema parseado.

---

## Fase 2 — Integração Typst (sem. 3–5)

**Objetivo:** dado um template e um payload, compilar e obter bytes de PDF.

- [ ] Adicionar dependências `typst` e `typst-pdf` ao `papelon-core`
- [ ] Implementar `TypstCompiler`:
  - [ ] Recebe `RenderContext` (mapa de caminho → bytes) + payload JSON
  - [ ] Injeta payload via `sys.inputs.data`
  - [ ] Retorna `Document` (bytes do PDF) ou `CompilationError`
  - [ ] Mapear erros do Typst para `CompilationError` com mensagem útil
- [ ] Implementar `RenderService`:
  - [ ] Recebe `Arc<dyn TemplateResolver>` no construtor
  - [ ] `render(request: RenderRequest) → Result<Document, RenderError>`
  - [ ] Valida payload contra schema antes de compilar
  - [ ] Monta `RenderContext` com os arquivos do `ResolvedTemplate`
  - [ ] Chama `TypstCompiler::compile`
- [ ] Testes de integração:
  - [ ] Template mínimo (só `#let data = json(bytes(sys.inputs.data))`) compila
  - [ ] Payload inválido retorna `ValidationError` antes de chamar o compilador
  - [ ] Erro de sintaxe no .typ retorna `CompilationError` com linha e mensagem

**Critério de saída:** teste que recebe um template .typ real e um JSON e produz bytes de PDF válidos (magic bytes `%PDF`).

---

## Fase 3 — Servidor HTTP (sem. 6–8)

**Objetivo:** expor o RenderService via HTTP com tratamento correto de erros.

- [ ] Criar `papelon-server` com `axum` e `tokio`
- [ ] Implementar rotas:
  - [ ] `POST /render/:template` → chama RenderService, retorna PDF ou erro JSON
  - [ ] `GET /templates` → lista templates disponíveis via TemplateResolver
  - [ ] `GET /healthz` → retorna `{"status": "ok"}`
- [ ] Mapeamento de erros de domínio para HTTP:
  - [ ] `TemplateNotFound` → 404
  - [ ] `ValidationError` → 422 com lista de violations
  - [ ] `CompilationError` → 500 com mensagem do Typst
- [ ] `main.rs` com parsing de argumentos via `clap`:
  - [ ] `--templates <path>` (obrigatório no Modo 1)
  - [ ] `<port>` (default 8080)
- [ ] Logs estruturados com `tracing`: cada render loga template, latência, tamanho do PDF
- [ ] Teste end-to-end: subir servidor em porta aleatória, fazer POST, verificar PDF

**Critério de saída:** `cargo run -- --templates ./examples/templates 8080` sobe o servidor e `curl -X POST http://localhost:8080/render/invoice -d @payload.json` retorna um PDF.

---

## Fase 4 — Deploy + DX (sem. 9–10)

**Objetivo:** qualquer dev consegue usar o projeto em menos de 5 minutos.

- [ ] Dockerfile multi-stage:
  - [ ] Stage 1: compilar o binário Rust
  - [ ] Stage 2: imagem final debian:slim com só o binário
  - [ ] Resultado: imagem < 100MB
  - [ ] `COPY templates/` no Dockerfile de exemplo
- [ ] `docker-compose.yml` de exemplo com volume de templates
- [ ] Modo `--watch` no servidor:
  - [ ] Usar crate `notify` para detectar mudanças na pasta de templates
  - [ ] Ao detectar mudança: re-escanear template alterado, atualizar `FilesystemResolver`
  - [ ] Logar qual template foi recarregado
- [ ] CLI mínima em `papelon-cli`:
  - [ ] `papelon init <name>`: cria `name/main.typ` e `name/schema.json` com boilerplate
  - [ ] Boilerplate do `main.typ` já inclui `#let data = json(bytes(sys.inputs.data))`
- [ ] README com:
  - [ ] O que é o projeto (3 parágrafos)
  - [ ] Quickstart: docker-compose + curl em 5 minutos
  - [ ] Referência do `schema.json`
  - [ ] Exemplo completo de template .typ
  - [ ] Referência da API HTTP
- [ ] Templates de exemplo no repositório:
  - [ ] `examples/templates/invoice/` — template completo com itens e totais
  - [ ] `examples/templates/welcome/` — template simples de boas-vindas

**Critério de saída (= critério do MVP):** README funciona — alguém sem contexto consegue subir o servidor e gerar um PDF seguindo o quickstart.

---

## Pós-MVP — Modo 2 (Store)

Não implementar até o Modo 1 estar em produção e com feedback de usuários reais.

- [ ] Definir `StoreBackend` trait
- [ ] Implementar `FilesystemStore`
- [ ] Implementar protocolo de push (blobs, manifests, refs)
- [ ] Implementar `StoreResolver`
- [ ] Implementar `S3Store`
- [ ] CLI: `papelon push`, `papelon tag`, `papelon ls`
- [ ] Rotas com tag: `POST /render/:template/:tag`