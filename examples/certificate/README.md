# Certificate example project

An A4 landscape certificate that bundles its own fonts, draws its border with Typst shapes, and
embeds an SVG seal. Two styles (`classic`, `modern`) switch colors and ornaments.

```text
typrest.toml
templates/
  certificate/             # template ID and route: POST /certificate
    main.typ
    schema.yml
    payload.json           # sample request body
    assets/seal.svg
    fonts/
      Cinzel.ttf           # headings (OFL, see OFL-Cinzel.txt)
      PinyonScript-Regular.ttf   # recipient name and signature (OFL, see OFL-PinyonScript.txt)
```

## Run it

```bash
cd examples/certificate
typrest check
typrest serve --watch
```

```bash
curl -X POST http://localhost:8080/certificate \
  -H 'content-type: application/json' \
  --data @templates/certificate/payload.json \
  -o certificate.pdf
```

Set `"style": "modern"` in the payload for the blue variant.

## What it demonstrates

- **Bundled fonts.** Lowercase `.ttf`/`.otf` files inside the template folder are available to that
  template only, and Typrest never reads operating-system fonts (`REQ-TPL-040`, `REQ-TPL-041`). The
  template simply asks for `"Cinzel"` and `"Pinyon Script"`. Confirm they are embedded:

  ```bash
  pdffonts certificate.pdf
  ```

  The output lists `Cinzel-Regular` and `PinyonScript-Regular`. A `DejaVuSansMono` entry may also
  appear: it is the Typst built-in fallback for the ornament glyph (`❖`, `◆`), which Cinzel lacks.
  Cinzel ships as a variable font; Typst uses its default (Regular) instance, and the bold course
  title is a synthesized bold.
- **Fonts are per template.** To share fonts with every template in a project, move the `fonts/`
  folder to `templates/_shared/` (`REQ-TPL-040`).
- **Landscape page.** `#set page(paper: "a4", flipped: true, margin: 0pt)`.
- **Shapes plus SVG.** The border, corner dots and header band are Typst `rect`/`circle` shapes;
  the seal is `assets/seal.svg`, read from the template directory.
- **Enum-driven style.** `style` (`classic` or `modern`, default `classic`) selects an entry of the
  `styles` dictionary in `main.typ`. See [Schema v1](../../docs/specs/schema-v1.md).

## Invalid payload

Omitting the required `certificate_id`:

```bash
curl -i -X POST http://localhost:8080/certificate \
  -H 'content-type: application/json' \
  --data '{"recipient":"A","course":"B","date":"C","issuer":{"name":"D"}}'
```

returns `422` with the structured envelope:

```json
{"error":"schema_validation_failed","message":"Payload validation failed","violations":[{"field":"$.certificate_id","issue":"required field missing"}]}
```

## Customizing

Add a style by adding an entry to `styles` in `main.typ` and to the `style` enum in `schema.yml`.
Replace `assets/seal.svg` with your own seal. To use another font, drop its `.ttf`/`.otf` and license
file into `fonts/` and change the `display` or `script` names in `main.typ`. Only bundle fonts
whose license allows redistribution.
