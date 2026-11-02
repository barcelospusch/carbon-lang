# Carbon 0.1.0

## Funcionalidades

- Compilador CLI Rust de `.cb` para PDF acadêmico com composição LaTeX/Tectonic.
- Metadados, três níveis de títulos, negrito, itálico, matemática TeX/AMS e letras gregas.
- Código cercado/indentado; tabelas booktabs, legendas e colunas configuráveis.
- PNG, JPEG e PDF vetorial, caminhos relativos, IDs e referências clicáveis.
- Blocos Mermaid renderizados como figuras vetoriais, com legenda e referência.
- Backend nativo printpdf opcional, com paginação A4 e fontes TrueType.
- Exemplos organizados, testes, workflows CI e empacotamento de releases com SHA-256.

## Dependências de execução

Tectonic para o backend padrão. Node >=22.13, Mermaid CLI e Chromium somente para diagramas Mermaid. O backend nativo requer fontes TrueType locais. Os arquivos de release não incluem esses runtimes.
