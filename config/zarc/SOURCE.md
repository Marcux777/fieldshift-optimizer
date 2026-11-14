# Procedência do recorte ZARC

- Conjunto: Tábua de Risco — Zoneamento Agrícola de Risco Climático (MAPA/SPA/CGRA), dados abertos, licença CC-BY.
  https://dados.agricultura.gov.br/dataset/tabua-de-risco-zoneamento-agricola-de-risco-climatico
- Arquivo nacional 2026/27: https://dados.agricultura.gov.br/dataset/6d3d141c-885e-41a4-ab7f-dc8ff323b96f/resource/139e5a60-1f43-4cc8-aeab-a35dbbf816c0/download/dados-abertos-tabua-de-risco-safra-2026-2027.csv
  baixado em 13/11/2026; SHA-256 do arquivo nacional: b5d242f78571d4694c843f8a5f8fcad4dc10c1c83192ebae2d40b78cc5f5efe3
- Recorte: cabeçalho + todas as linhas com `geocodigo` = 5218805 (Rio Verde, GO), sem outra alteração. 251 linhas de dados.
  Copiado de `docs/planning/prep/zarc-rio-verde/` em 14/11/2026 (hash conferido com o `SHA256SUMS` de lá).
- Dicionário de dados: https://dados.agricultura.gov.br/dataset/6d3d141c-885e-41a4-ab7f-dc8ff323b96f/resource/bebb0ebb-bc75-460c-b900-a7a1ebd87bee/download/dicionario-de-dados-tabua-de-risco-2026.pdf
- Códigos: `Cod_Ciclo` 20/21/22 = Grupo I/II/III; `Cod_Solo` 1/2/3 = arenoso/textura média/argiloso, 11–16 = AD1–AD6; `Cod_Outros_Manejos` 1 = Sequeiro; `dec1..dec36` = classe de risco (%) por decêndio de semeadura, 0 = não indicado.
- Portarias de origem das linhas usadas (coluna `Portaria`): Soja Port.198 (03/07/2026); Milho 1ª Port.221 (10/07/2026); Milho 2ª Port.322 (30/07/2026); Milho 2ª + braquiária Port.337 (06/08/2026); Sorgo granífero 2ª Port.273 (22/07/2026).
- Nota de download: o servidor recusa o agente padrão do curl; foi usado `--http1.1` e um User-Agent de navegador.
- Uso no FieldShift: a tábua 2026/27 é aplicada como calendário fixo a todos os anos históricos (1981–2025). Isso é premissa declarada: o ZARC foi calibrado com séries que se sobrepõem aos anos de teste (vazamento residual, só declarável).
