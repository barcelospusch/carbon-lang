# Release no GitHub

O workflow `.github/workflows/release.yml` cria **um rascunho**, com binários e SHA-256, após os checks. A publicação continua manual. Não há upload de crates.io (`publish = false`).

## Preparar

1. Crie o repositório GitHub e envie o projeto. Nenhuma URL ou nome de organização foi presumido nos workflows.
2. Mantenha `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md` e a documentação sincronizados. A versão atual é `0.1.2`.
3. Execute `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` e `cargo test --locked`. Instale os runtimes e execute também os testes ignorados e o script de exemplos.
4. Confirme os resultados da CI nos três sistemas. O job MSRV verifica Rust 1.88; a integração completa com Tectonic/Mermaid é feita no Windows.

## Disparar o rascunho

Na raiz de um checkout Git, após revisar e commitar os arquivos:

```sh
git tag -a v0.1.2 -m "Carbon v0.1.2"
git push origin v0.1.2
```

Também é possível executar **Draft release** manualmente em Actions, informando uma tag já existente. O workflow recusa tags que não correspondam à versão do Cargo ou releases já publicadas. Na execução manual, os checks e builds fazem checkout explícito da tag informada para validar o código que será distribuído.

O workflow usa `GITHUB_TOKEN`, sem secret pessoal. Apenas o job que cria o rascunho possui `contents: write`. Os demais jobs têm leitura. Para publicar, revise o rascunho na aba Releases e clique em Publish release.

## Artefatos

- Windows x86_64: ZIP.
- Linux x86_64 (glibc, runner Ubuntu 24.04): tar.gz.
- macOS Apple Silicon (aarch64): tar.gz.
- Um arquivo `.sha256` por arquivo compactado.

Cada pacote inclui o executável, documentação, exemplos e instaladores/manifestos dos runtimes opcionais. Não inclui Tectonic, Node, node_modules, Chromium ou caches. O usuário instala Tectonic para o backend padrão e Node/Mermaid/Chromium somente se usar diagramas. Distribuições Linux mais antigas que o ambiente de build podem exigir compilação local.

Para testar o pacote local no Windows:

```powershell
cargo build --release --locked
python scripts/package-release.py --version v0.1.2 --target x86_64-pc-windows-msvc --binary target/release/carbon.exe
```

Extraia o ZIP em outra pasta e confira `carbon.exe --help`. Consulte README.md para instalação dos runtimes. Os workflows foram preparados no projeto; a execução dos runners e o upload só acontecem após o envio ao GitHub.

## Primeiro envio do projeto

Esta pasta foi preparada sem presumir um repositório remoto. Caso ainda não seja um checkout Git, crie o repositório no GitHub e execute, substituindo a URL pelo seu remoto:

```sh
git init
git add .
git commit -m "Prepare Carbon v0.1.2"
git branch -M main
git remote add origin URL_DO_SEU_REPOSITORIO
git push -u origin main
```

Depois que a CI passar, crie e envie a tag conforme as instruções acima. Os scripts de empacotamento incluem os fontes, avisos de licença encontrados nas dependências Rust e um índice `licenses/dependencies.json`, além dos demais arquivos do projeto. Nenhuma etapa local desta preparação envia commits, tags ou releases ao GitHub.
