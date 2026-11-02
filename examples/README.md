# Exemplos Carbon

Cada pasta contém documentos independentes e, quando necessário, assets locais.

- `basico/artigo.cb`: texto e blocos básicos.
- `matematica/matematica.cb`: notação científica.
- `imagens/figuras.cb`: imagens e referências, com assets em `imagens/figuras/`.
- `mermaid/diagramas.cb`: diagramas Mermaid; requer o runtime opcional.
- `referencia/trecho.cb`: comparação com o artigo LaTeX original.
- `templates/modelo-cientifico.tex`: modelo LaTeX adicional, fora da gramática Carbon.

Execute `python scripts/check-examples.py --binary target/release/carbon.exe` (Windows) ou a variante sem `.exe` para compilar todos os `.cb` em `output/examples/`.
