# DECISIONS.md — Log de Decisões Arquiteturais

Registro de decisões importantes com contexto e alternativas consideradas.
Adicionar uma entrada sempre que uma decisão não-óbvia for tomada.

---

## DEC-001 — Typst como engine de compilação

**Data:** 2025-11  
**Status:** Aceita

**Decisão:** Usar Typst como engine de geração de PDF, integrado via crate Rust (`typst` + `typst-pdf`), não como binário externo.

**Contexto:** O mercado de geração de PDF server-side tem dois problemas principais: soluções browser-based (Puppeteer) são pesadas e complexas de escalar; libs específicas por linguagem (pdfmake, ReportLab) criam lock-in de ecossistema.

**Alternativas consideradas:**
- Puppeteer/Chrome headless: ~300MB de overhead, complexo para escalar, depende de browser
- LaTeX: distribuição >1GB, lento, sintaxe hostil
- wkhtmltopdf: abandonado oficialmente
- Invocar `typst` como processo filho: mais simples inicialmente, mas cria dependência de binário externo no container e overhead de processo por render

**Por que Typst via crate:**
- Binário único ~40MB
- Compila em milissegundos
- Escrito em Rust — integração nativa sem overhead de FFI ou processo filho
- Suporte nativo a JSON via `sys.inputs`
- Crescimento rápido (2ª linguagem mais crescida no GitHub em 2025)

**Consequências:** A integração com a crate `typst` tem API não-trivial (projetada para o editor web). Reservar tempo extra na Fase 2 do roadmap.

---

## DEC-002 — Dois modos de operação via trait, não via flag condicional

**Data:** 2025-11  
**Status:** Aceita

**Decisão:** Os dois modos (Filesystem e Store) são implementados como implementações diferentes do trait `TemplateResolver`, não como branches condicionais espalhados pelo código.

**Contexto:** O MVP implementa apenas o Modo 1 (Filesystem). O Modo 2 (Store) é planejado para a versão seguinte. A separação precisa estar no código desde o início para não exigir refatoração pesada depois.

**Alternativas consideradas:**
- Feature flags Cargo: mais explícito em compile time, mas cria build separados
- Enum com variantes: mais simples, mas torna o código do servidor ciente dos modos
- Trait (escolhida): o servidor HTTP conhece apenas `Arc<dyn TemplateResolver>`, qualquer implementação funciona

**Consequências:** `papelon-core` tem o trait `TemplateResolver`. `papelon-server` recebe a implementação concreta via injeção na inicialização. Testes de core usam `MemoryResolver`.

---

## DEC-003 — Templates como parte do container no Modo 1

**Data:** 2025-11  
**Status:** Aceita

**Decisão:** No Modo 1, templates são copiados para dentro da imagem Docker via `COPY templates/`. Atualizar um template implica rebuild e redeploy do container.

**Contexto:** Esse é o trade-off central do Modo 1 vs Modo 2.

**Por que é aceitável no MVP:**
- É o mesmo modelo de qualquer aplicação com arquivos estáticos (migrations, configs, assets)
- Elimina toda a complexidade de Store, push, e versionamento
- Para a maioria dos casos de uso iniciais (equipes pequenas, poucos templates), o ciclo de deploy já existe e é aceitável
- A dor de "atualizar template sem redeploy" só aparece quando o usuário já está convencido do produto

**Quando o Modo 2 faz sentido:** templates que mudam com frequência independente do código, times com múltiplos ambientes (staging/prod) que precisam de rollback sem redeploy, microserviços que precisam compartilhar templates.

---

## DEC-004 — `_shared/` é resolvido em tempo de leitura, não de compilação

**Data:** 2025-11  
**Status:** Aceita

**Decisão:** O `FilesystemResolver` inclui os arquivos do `_shared/` no `ResolvedTemplate` de cada template durante a leitura/watch, não durante a compilação.

**Por que:** O `RenderContext` montado para o Typst precisa ser autocontido. Se `_shared/` fosse resolvido na compilação, o compilador precisaria conhecer a estrutura de diretórios do projeto — quebrando o isolamento entre modos.

**Consequências:** O `ResolvedTemplate` contém um mapa completo de todos os arquivos necessários, incluindo os do `_shared/`. Mudança em `_shared/` no modo `--watch` força reload de todos os templates que o referenciam.

---

## DEC-005 — Injeção de dados via `sys.inputs`, não via arquivo

**Data:** 2025-11  
**Status:** Aceita

**Decisão:** O payload JSON é passado para o Typst via `--input data=<json_string>` (mapeado para `sys.inputs.data` no template), não gravando um arquivo `input.json` em disco.

**Alternativa considerada:** Gravar `input.json` no tmpdir e o template faz `#let data = json("input.json")`. Mais familiar para quem conhece Typst, mas exige tmpdir em disco.

**Por que sys.inputs:** Sem I/O de disco por render (exceto o PDF de saída), mais limpo, e o compilador Typst via crate suporta inputs em memória diretamente.

**Consequência para o usuário:** Todo template deve começar com `#let data = json(bytes(sys.inputs.data))`. Documentar claramente no README e no boilerplate gerado pelo `papelon init`.

---

## Template para novas decisões

```
## DEC-XXX — Título

**Data:** YYYY-MM  
**Status:** Aceita | Em discussão | Rejeitada

**Decisão:** O que foi decidido.

**Contexto:** Por que essa decisão foi necessária.

**Alternativas consideradas:**
- Opção A: ...
- Opção B: ...

**Por que a escolha feita:** ...

**Consequências:** O que muda no código ou no produto.
```