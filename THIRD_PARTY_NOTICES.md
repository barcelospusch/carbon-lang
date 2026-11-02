# Dependências e licenças de terceiros

O código Carbon é distribuído sob a licença MIT (veja LICENSE).

As bibliotecas Rust são declaradas em Cargo.toml e fixadas em Cargo.lock. As licenças declaradas por elas podem ser inspecionadas com `cargo metadata --locked --format-version 1`.

O renderizador Mermaid opcional é o projeto @mermaid-js/mermaid-cli (MIT), instalado separadamente. O lockfile em tools/mermaid fixa suas dependências. Mermaid CLI, Mermaid, Puppeteer, Chromium e fontes fornecidas por esses projetos conservam suas próprias licenças e avisos.

Tectonic é instalado separadamente de suas releases oficiais. Seus componentes TeX, pacotes e fontes mantêm suas licenças próprias. Consulte https://github.com/tectonic-typesetting/tectonic e https://github.com/mermaid-js/mermaid-cli para os avisos dos respectivos projetos.

O empacotamento do Carbon exclui Tectonic, node_modules, Chromium, caches TeX e fontes proprietárias do sistema. Os exemplos de gráficos sintéticos em examples/imagens são arquivos de demonstração criados para este projeto. Os arquivos de referência em examples/referencia foram fornecidos pelo autor do projeto, preservados como material de referência.
