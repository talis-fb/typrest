# Typst Integration Reference

Referência curada para integração com o compilador Typst via crate Rust.
Este arquivo existe para dar contexto à LLM sobre a API do Typst — não clonar o repositório inteiro.

Fonte principal: https://typst.app/blog/2025/automated-generation/
Repositório de exemplo: https://github.com/tfachmann/typst-as-library

---

## Como o Typst recebe dados externos

O mecanismo oficial para injetar dados em um template Typst é `sys.inputs`.
No CLI seria: `typst compile --input data='{"name":"João"}' main.typ`
Via crate Rust, o mesmo é feito passando um `HashMap<String, String>` de inputs.

No template, o dado é lido assim:
```typst
#let data = json(bytes(sys.inputs.data))
```

`sys.inputs.data` é uma string. `bytes()` converte para bytes. `json()` parseia o JSON.
Este é o padrão canônico — **todo template neste projeto deve começar com esta linha**.

---

## Dependências no Cargo.toml

```toml
[dependencies]
typst = "0.13"        # ou a versão mais recente — verificar crates.io
typst-pdf = "0.13"    # exportador PDF — versão deve coincidir com typst
```

Versão atual estável: verificar https://crates.io/crates/typst

---

## O trait `World` — conceito central

A crate `typst` não lê arquivos do filesystem por conta própria.
O chamador implementa o trait `World` que responde perguntas do compilador:
- "qual é o conteúdo do arquivo X?"
- "qual é a fonte Y?"
- "qual é a data/hora atual?"

Este é o ponto de extensão principal. Para este projeto, o `World` recebe
um `HashMap<PathBuf, Bytes>` com todos os arquivos do template já em memória
(vindos do `RenderContext`).

### Interface mínima do trait `World`

```rust
use typst::World;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::foundations::Bytes;
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::Library;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct PapelonWorld {
    /// Fonte raiz do documento (main.typ)
    main: Source,
    /// Todos os arquivos do template: caminho → bytes
    files: HashMap<PathBuf, Bytes>,
    /// Biblioteca padrão do Typst (funções built-in)
    library: LazyHash<Library>,
    /// Índice de fontes disponíveis
    book: LazyHash<FontBook>,
    /// Fontes carregadas
    fonts: Vec<Font>,
}

impl World for PapelonWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        // retorna o source do arquivo pelo FileId
        // main.typ é o caso principal
        // imports de outros .typ também passam por aqui
        todo!()
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        // retorna bytes de arquivos não-typ (imagens, fontes, etc.)
        todo!()
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<i64>) -> Option<typst::foundations::Datetime> {
        // retornar None é aceitável — templates que usam `datetime.today()` vão falhar
        // para MVP retornar a data atual do sistema
        None
    }
}
```

---

## Compilação e exportação para PDF

```rust
use typst::compile;
use typst_pdf::{pdf, PdfOptions};

// 1. Criar o World com os arquivos do template
let world = PapelonWorld::new(files, input_json);

// 2. Compilar — retorna Result com o documento ou erros
let result = compile(&world);

match result.output {
    Ok(document) => {
        // 3. Exportar para PDF
        let options = PdfOptions::default();
        let pdf_bytes = pdf(&document, &options)?;
        // pdf_bytes: Vec<u8> — os bytes do PDF
    }
    Err(errors) => {
        // errors: Vec<SourceDiagnostic>
        // cada erro tem: message, span (localização no .typ), severity
        for error in &errors {
            eprintln!("Typst error: {}", error.message);
        }
    }
}
```

**Nota:** `compile()` retorna um `SourceResult` que tem tanto `output` (pode ser Ok mesmo com warnings)
quanto `warnings`. Sempre verificar `output` — warnings não impedem a geração do PDF.

---

## Passando inputs (sys.inputs) via crate

Os inputs são passados como parte da `Library` na construção do World:

```rust
use typst::Library;
use typst::foundations::Dict;

// Construir os inputs como Dict do Typst
let mut inputs = Dict::new();
inputs.insert(
    "data".into(),
    // o payload JSON serializado como string
    serde_json::to_string(&payload).unwrap().into()
);

// Criar a biblioteca com os inputs
let library = Library::builder()
    .with_inputs(inputs)
    .build();
```

O template então acessa via `sys.inputs.data`.

---

## Carregando fontes

O Typst precisa de fontes para compilar. Sem fontes, textos não renderizam.

```rust
use typst::text::{Font, FontBook};

// Opção A: Embedar fontes padrão (Linux Libertine, etc.)
// A crate `typst-assets` provê as fontes padrão do Typst
// [dependencies]
// typst-assets = { version = "0.13", features = ["fonts"] }

use typst_assets::fonts;

fn load_default_fonts() -> (FontBook, Vec<Font>) {
    let mut book = FontBook::new();
    let mut fonts = Vec::new();
    
    for data in fonts() {
        let buffer = Bytes::from_static(data);
        for font in Font::iter(buffer) {
            book.push(font.info().clone());
            fonts.push(font);
        }
    }
    
    (book, fonts)
}

// Opção B: Carregar fontes customizadas do template (assets/)
// Os bytes da fonte vêm dos blobs do template — já estão em memória
fn load_font_from_bytes(data: Bytes, book: &mut FontBook, fonts: &mut Vec<Font>) {
    for font in Font::iter(data) {
        book.push(font.info().clone());
        fonts.push(font);
    }
}
```

**Recomendação para este projeto:**
- Sempre incluir as fontes padrão do Typst (`typst-assets`)
- Fontes customizadas do template (em `assets/`) são carregadas dos blobs do RenderContext

---

## Resolvendo imports entre arquivos

Quando o template tem `#import "partials/header.typ"`, o Typst chama `World::source()`
com o `FileId` do arquivo importado. Precisamos mapear esse FileId para o conteúdo correto.

```rust
use typst::syntax::{FileId, VirtualPath, Source};
use typst::diag::{FileError, FileResult};

impl World for PapelonWorld {
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            return Ok(self.main.clone());
        }
        
        // Extrair o caminho do FileId e buscar nos arquivos do template
        let path = id.vpath().as_rootless_path();
        
        match self.files.get(path) {
            Some(bytes) => {
                let text = std::str::from_utf8(bytes)
                    .map_err(|_| FileError::InvalidUtf8)?;
                Ok(Source::new(id, text.to_string()))
            }
            None => Err(FileError::NotFound(path.to_path_buf())),
        }
    }
    
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = id.vpath().as_rootless_path();
        self.files.get(path)
            .cloned()
            .ok_or_else(|| FileError::NotFound(path.to_path_buf()))
    }
}
```

---

## Criando o FileId para o main.typ

```rust
use typst::syntax::{FileId, VirtualPath};

// O main.typ é sempre a raiz — caminho virtual "/"
let main_id = FileId::new(None, VirtualPath::new("/main.typ"));

// Ler o conteúdo do main.typ dos arquivos do template
let main_content = files
    .get(Path::new("main.typ"))
    .expect("main.typ deve existir no template");

let main_text = std::str::from_utf8(main_content).expect("main.typ deve ser UTF-8");
let main_source = Source::new(main_id, main_text.to_string());
```

---

## Extraindo erros de compilação para mensagens úteis

```rust
use typst::diag::SourceDiagnostic;

fn format_typst_error(error: &SourceDiagnostic, world: &impl World) -> String {
    let message = error.message.to_string();
    
    // Tentar obter informação de localização
    if let Some(range) = world.source(error.span.id().unwrap_or(world.main()))
        .ok()
        .and_then(|s| s.range(error.span))
    {
        format!("line {}: {}", range.start.line + 1, message)
    } else {
        message
    }
}
```

---

## Exemplo mínimo completo — World funcional

Este é o exemplo do repositório `typst-as-library` adaptado para o contexto do projeto:

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use typst::foundations::{Bytes, Dict};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst::diag::{FileError, FileResult};

pub struct TemplateWorld {
    main: Source,
    files: HashMap<PathBuf, Bytes>,
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

impl TemplateWorld {
    pub fn new(
        files: HashMap<PathBuf, Bytes>,
        input_json: &str,
    ) -> Self {
        // Inputs
        let mut inputs = Dict::new();
        inputs.insert("data".into(), input_json.into());
        
        let library = LazyHash::new(
            Library::builder().with_inputs(inputs).build()
        );
        
        // Fontes
        let mut book = FontBook::new();
        let mut fonts = Vec::new();
        for data in typst_assets::fonts() {
            let buffer = Bytes::from_static(data);
            for font in Font::iter(buffer) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }
        
        // main.typ
        let main_id = FileId::new(None, VirtualPath::new("/main.typ"));
        let main_bytes = files.get(Path::new("main.typ")).expect("main.typ required");
        let main_text = std::str::from_utf8(main_bytes).expect("main.typ must be UTF-8");
        let main = Source::new(main_id, main_text.to_string());
        
        Self {
            main,
            files,
            library,
            book: LazyHash::new(book),
            fonts,
        }
    }
}

impl World for TemplateWorld {
    fn library(&self) -> &LazyHash<Library> { &self.library }
    fn book(&self) -> &LazyHash<FontBook> { &self.book }
    fn main(&self) -> FileId { self.main.id() }
    
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() { return Ok(self.main.clone()); }
        let path = id.vpath().as_rootless_path();
        let bytes = self.files.get(path).ok_or_else(|| FileError::NotFound(path.into()))?;
        let text = std::str::from_utf8(bytes).map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, text.to_string()))
    }
    
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = id.vpath().as_rootless_path();
        self.files.get(path).cloned().ok_or_else(|| FileError::NotFound(path.into()))
    }
    
    fn font(&self, index: usize) -> Option<Font> { self.fonts.get(index).cloned() }
    
    fn today(&self, _offset: Option<i64>) -> Option<typst::foundations::Datetime> { None }
}

// Uso:
pub fn compile_template(
    files: HashMap<PathBuf, Bytes>,
    payload: &serde_json::Value,
) -> Result<Vec<u8>, String> {
    let input_json = serde_json::to_string(payload).unwrap();
    let world = TemplateWorld::new(files, &input_json);
    
    let result = typst::compile(&world);
    
    match result.output {
        Ok(document) => {
            let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
                .map_err(|e| format!("PDF export error: {:?}", e))?;
            Ok(pdf)
        }
        Err(errors) => {
            let messages: Vec<String> = errors.iter()
                .map(|e| e.message.to_string())
                .collect();
            Err(messages.join("; "))
        }
    }
}
```

---

## Crates adicionais relevantes

```toml
[dependencies]
typst = "0.13"
typst-pdf = "0.13"
typst-assets = { version = "0.13", features = ["fonts"] }  # fontes padrão embedadas
```

- `typst-assets`: provê as fontes padrão (Linux Libertine, New Computer Modern, etc.)
  que o Typst usa quando nenhuma fonte é especificada no template.
  **Sem isso, templates sem `#set text(font: "...")` explícito não compilam.**

---

## Armadilhas conhecidas

**API instável entre versões:** A crate `typst` não tem garantias de estabilidade semântica.
Entre versões minor pode haver breaking changes. Fixar a versão exata no `Cargo.toml`.

**`PdfOptions` mudou entre versões:** Em versões antigas era `pdf(&document, None, None)`.
Em versões mais recentes é `pdf(&document, &PdfOptions::default())`. Verificar a assinatura
na versão que estiver usando.

**Compilação thread-safe:** O `World` precisa implementar `Send + Sync` para uso em
servidores async. Usar `Arc<RwLock<>>` se houver estado mutável.

**Sem filesystem access automático:** O Typst via crate não lê nada do disco por conta própria.
Todo arquivo que o template precisar (imagens, fontes, imports) deve estar no `HashMap` passado
ao World. Esta é a propriedade que garante isolamento entre compilações.

**Pacotes do Typst Universe:** Templates que usam `#import "@preview/..."` precisam que
o conteúdo do pacote esteja disponível via `World::file()`. Para o MVP, não suportar
pacotes externos — retornar `FileError::NotFound` e documentar a limitação.