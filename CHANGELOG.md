# Carbon 0.1.2

## Correções

- Normaliza caminhos Windows do Mermaid CLI antes de passá-los ao Node 22, evitando `EISDIR: lstat D:` no GitHub Actions.
- Executa testes de integração em sequência para evitar disputa na inicialização do cache de formatos do Tectonic.
- Inclui um teste de regressão que executa uma entrada Node por caminho com espaços a partir de outro diretório.
- Mantém o teste de tabelas independente dos documentos privados de referência.
- Alinha a versão do Cargo à próxima tag de release.

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
