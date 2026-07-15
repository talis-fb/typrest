# Especificação do Schema (`schema.json`) - Versão 1

## Objetivo

O arquivo `schema.json` define o contrato esperado para um valor JSON.

Na versão `v1`, apenas a validação do corpo da requisição (`request.body`) será suportada.

Todo schema é composto por um objeto raiz contendo uma versão e um schema para o body.

O campo `schemaVersion` é obrigatório.

Exemplo:

```json
{
  "schemaVersion": "v1",
  "request": {
    "body": {
      "type": "object",
      "properties": {
        "name": {
          "type": "string",
          "required": true
        }
      }
    }
  }
}
```

---

# Estrutura

```text
schema.json
 ├── schemaVersion (string)
 └── request
      └── body (Schema)
```

O campo `body` é sempre um `Schema`.

Qualquer lugar da linguagem que espera um `Schema` utiliza exatamente a mesma estrutura.

---

# Schema

Um Schema é um objeto que descreve como validar um único valor JSON.

Todos os campos do schema são opcionais.

Um schema vazio:

```json
{}
```

é válido e aceita qualquer valor.

---

# Campo `required`

Tipo:

```text
boolean
```

Padrão:

```text
false
```

Responsabilidade:

Determina se o valor precisa existir.

Regras:

- `required` verifica apenas existência.
- Se o valor não existir (`undefined`) e `required` for `false`, o schema é considerado válido.
- Se o valor não existir e `required` for `true`, a validação falha.
- Se o valor existir (inclusive `null`), a validação continua normalmente.

---

# Campo `type`

Tipo:

```text
string
```

Obrigatório:

Não.

Valores suportados:

- string
- number
- boolean
- object
- array
- null

Responsabilidade:

Verifica o tipo do valor.

Regras:

- Um schema pode possuir apenas um único tipo.
- Não é permitido utilizar uma lista de tipos.
- Caso o valor exista, seu tipo deve corresponder exatamente ao definido em `type`.
- Caso `type` não seja informado, nenhuma validação de tipo é realizada.

---

# Campo `properties`

Disponível apenas quando:

```text
type == object
```

Tipo:

```text
object<string, Schema>
```

Cada propriedade representa o schema de um campo do objeto.

Exemplo:

```json
{
  "type": "object",
  "properties": {
    "name": {
      "type": "string"
    },
    "age": {
      "type": "number"
    }
  }
}
```

É erro de schema utilizar `properties` quando `type` não for `object`.

---

# Campo `items`

Disponível apenas quando:

```text
type == array
```

Tipo:

```text
Schema
```

Define o schema de cada elemento do array.

Exemplo:

```json
{
  "type": "array",
  "items": {
    "type": "string"
  }
}
```

É erro de schema utilizar `items` quando `type` não for `array`.

---

# Campo `enum`

Tipo:

```text
array
```

Responsabilidade:

Restringe o valor para um conjunto fixo de possibilidades.

Regras:

- Todos os valores do enum devem possuir exatamente o mesmo tipo.
- Se `type` estiver definido, todos os valores do enum devem possuir exatamente esse tipo.
- Caso exista qualquer divergência de tipos, o schema é inválido.
- A comparação é feita por igualdade exata.

Exemplo:

```json
{
  "type": "string",
  "enum": [
    "BRL",
    "USD"
  ]
}
```

Exemplo inválido:

```json
{
  "type": "string",
  "enum": [
    "BRL",
    10
  ]
}
```

---

# Campo `default`

Responsabilidade:

Define um valor padrão para o campo.

Regras:

- O valor definido em `default` deve possuir exatamente o mesmo tipo definido em `type`.
- Caso contrário, o schema é inválido.
- O valor padrão é aplicado apenas quando o campo não existir (`undefined`).
- O valor padrão nunca substitui `null`.
- Após aplicar o valor padrão, a validação continua normalmente.

Exemplo:

Entrada:

```json
{}
```

Schema:

```json
{
  "type": "string",
  "default": "anonymous"
}
```

Resultado:

```json
"anonymous"
```

Entrada:

```json
null
```

Resultado:

```json
null
```

---

# Ordem de validação

Todo schema deve ser processado exatamente na seguinte ordem:

1. Verificar existência (`required`).
2. Aplicar `default` caso o valor seja `undefined`.
3. Validar o tipo (`type`).
4. Validar `enum`.
5. Se `type == object`, validar `properties`.
6. Se `type == array`, validar `items`.

Nenhuma implementação deve alterar essa ordem.

---

# Regras de consistência

Os seguintes casos tornam o schema inválido antes da execução:

- `properties` utilizado sem `type: "object"`.
- `items` utilizado sem `type: "array"`.
- `default` incompatível com `type`.
- Algum valor de `enum` incompatível com `type`.
- Valores de tipos diferentes dentro de `enum`.
- Valor inválido para `type`.

Esses erros devem ser detectados durante o carregamento do schema, nunca durante a validação do payload.