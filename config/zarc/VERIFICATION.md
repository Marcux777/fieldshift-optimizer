# Tabela de verificação manual — ZARC 20%, Rio Verde, solo padrão (AD4 + argilosa)

Para os testes do Optimization (`tests/test_catalog.py`). Calculada em 14/11/2026 por um script descartável em `/tmp` (fora do repositório), lendo **só** `zarc-tabua-rio-verde-5218805-2026-2027.csv` (SHA-256 em `SHA256SUMS`) com as regras de `config/catalog.toml`. O código do motor deve reproduzir estes números exatamente; divergência = bug ou regra lida de outro modo (avisar o Methods).

Convenções usadas (todas declaradas em `catalog.toml`): Grupo I (`Cod_Ciclo` 20), sequeiro (`Cod_Outros_Manejos` 1), decêndio permitido só se `decN == "20"`; J1 semeia no 1º dia do decêndio; emergência 7 d; colheita = semeadura + 7 + ciclo (soja 100, milho 1ª 100, milho 2ª 110, milho 2ª + braquiária 115, sorgo 110); margem 0 d; J2 = 1º dia ≥ colheita de J1 num decêndio a 20% da 2ª safra; soja só em decêndios inteiros dentro de 25/set–02/jan. **t** = dia-índice a partir de 1/set (t = 0), calendário de 365 dias sem 29/fev; datas no ciclo 2026/27. J3/J4 = mesmas linhas com t + 365.

## 1. Decêndios a 20% por cultura no solo padrão

| Cultura | Chave de solo | Decêndios a 20% usados | Datas de semeadura |
|---|---|---|---|
| Soja GI (J1/J3) | AD4 (Cod_Solo 14) | dec28–dec36 (dec1 a 20% excluído pela regra legal; dec27 a 40%) | 01/10, 11/10, …, 21/12 |
| Milho 1ª GI (J1/J3) | AD4 (14) | dec30–dec36 | 21/10 … 21/12 |
| Milho 2ª GI (J2/J4) | AD4 (14) | dec1–dec4 | até 10/02 |
| Milho 2ª + braquiária GI (J2/J4) | AD4 (14) | dec1–dec4 | até 10/02 |
| Sorgo granífero 2ª GI (J2/J4) | argilosa (Cod_Solo 3) | dec1–dec8 | até 20/03 |

## 2. Pares J1 → J2 no solo padrão (semeadura → colheita da 2ª safra)

| J1 | Decêndio | Semeadura J1 | Colheita J1 | Milho 2ª | Milho 2ª + braquiária | Sorgo 2ª | Opções em J2 |
|---|---|---|---|---|---|---|---|
| soja | dec28 | 01/10 (t=30) | 16/01 (t=137) | 16/01 (t=137, dec2) → 13/05 (t=254) | 16/01 (t=137, dec2) → 18/05 (t=259) | 16/01 (t=137, dec2) → 13/05 (t=254) | 3 |
| soja | dec29 | 11/10 (t=40) | 26/01 (t=147) | 26/01 (t=147, dec3) → 23/05 (t=264) | 26/01 (t=147, dec3) → 28/05 (t=269) | 26/01 (t=147, dec3) → 23/05 (t=264) | 3 |
| soja | dec30 | 21/10 (t=50) | 05/02 (t=157) | 05/02 (t=157, dec4) → 02/06 (t=274) | 05/02 (t=157, dec4) → 07/06 (t=279) | 05/02 (t=157, dec4) → 02/06 (t=274) | 3 |
| soja | dec31 | 01/11 (t=61) | 16/02 (t=168) | **inviável** | **inviável** | 16/02 (t=168, dec5) → 13/06 (t=285) | 1 |
| soja | dec32 | 11/11 (t=71) | 26/02 (t=178) | **inviável** | **inviável** | 26/02 (t=178, dec6) → 23/06 (t=295) | 1 |
| soja | dec33 | 21/11 (t=81) | 08/03 (t=188) | **inviável** | **inviável** | 08/03 (t=188, dec7) → 03/07 (t=305) | 1 |
| soja | dec34 | 01/12 (t=91) | 18/03 (t=198) | **inviável** | **inviável** | 18/03 (t=198, dec8) → 13/07 (t=315) | 1 |
| soja | dec35 | 11/12 (t=101) | 28/03 (t=208) | **inviável** | **inviável** | **inviável** | pousio |
| soja | dec36 | 21/12 (t=111) | 07/04 (t=218) | **inviável** | **inviável** | **inviável** | pousio |
| milho_1a | dec30 | 21/10 (t=50) | 05/02 (t=157) | 05/02 (t=157, dec4) → 02/06 (t=274) | 05/02 (t=157, dec4) → 07/06 (t=279) | 05/02 (t=157, dec4) → 02/06 (t=274) | 3 |
| milho_1a | dec31 | 01/11 (t=61) | 16/02 (t=168) | **inviável** | **inviável** | 16/02 (t=168, dec5) → 13/06 (t=285) | 1 |
| milho_1a | dec32 | 11/11 (t=71) | 26/02 (t=178) | **inviável** | **inviável** | 26/02 (t=178, dec6) → 23/06 (t=295) | 1 |
| milho_1a | dec33 | 21/11 (t=81) | 08/03 (t=188) | **inviável** | **inviável** | 08/03 (t=188, dec7) → 03/07 (t=305) | 1 |
| milho_1a | dec34 | 01/12 (t=91) | 18/03 (t=198) | **inviável** | **inviável** | 18/03 (t=198, dec8) → 13/07 (t=315) | 1 |
| milho_1a | dec35 | 11/12 (t=101) | 28/03 (t=208) | **inviável** | **inviável** | **inviável** | pousio |
| milho_1a | dec36 | 21/12 (t=111) | 07/04 (t=218) | **inviável** | **inviável** | **inviável** | pousio |

Casos-chave para teste:
- **Soja → milho 2ª em AD4: viável só com soja em dec28, dec29 ou dec30.** Soja dec31 em diante: milho 2ª inviável (colheita 16/02 > fim de dec4 = 10/02).
- **Milho 1ª → milho 2ª em AD4: viável só com milho 1ª em dec30.**
- Soja ou milho 1ª em dec35–dec36: nenhuma 2ª safra viável → J2 = pousio (único caso em que o pousio é permitido).
- **P0 padrão** (soja dec29 → milho 2ª): viável em AD4 (J2 semeia 26/01, t = 147, dec3; colheita 23/05, t = 264). Em **AD1 o P0 é inviável** (soja dec29 está a 30%).
- Maior colheita de 2ª safra: 13/07 (t = 315) < 1/set → nunca sobrepõe J3 (asserção S2).
- A 2ª safra sempre semeia no próprio dia da colheita de J1 com margem 0, porque os decêndios a 20% de J2 são contíguos a partir de dec1.

## 3. Tamanho do conjunto viável |X| (regra: pousio só se nenhuma 2ª safra for viável)

Módulos por ano = Σ sobre opções de J1 de max(nº de 2ª safras viáveis, 1). |X| = módulos² (J3/J4 repetem o calendário). Entre parênteses: opções de J1 que só admitem pousio.

| AD \ textura | arenosa | média | argilosa |
|---|---|---|---|
| AD1 | 13 (10) → 169 | 13 (8) → 169 | 13 (4) → 169 |
| AD2 | 17 (10) → 289 | 17 (8) → 289 | 17 (4) → 289 |
| AD3 | 20 (10) → 400 | 20 (8) → 400 | 20 (4) → 400 |
| AD4 | 24 (10) → 576 | 24 (8) → 576 | 24 (4) → 576 |
| AD5 | 27 (10) → 729 | 27 (8) → 729 | 27 (4) → 729 |
| AD6 | 31 (10) → 961 | 31 (8) → 961 | 31 (4) → 961 |

No solo padrão: **24 módulos/ano, |X| = 576 planos**, 4 opções de J1 com pousio forçado. |X| não depende da textura (só muda a composição: sem sorgo, a opção de J1 vira pousio e continua contando 1).

## 4. Decêndios a 20% em todas as classes (para testes de solo)

| Classe | Soja (dec28–36) | Milho 1ª (dec25–36) | Milho 2ª | Milho 2ª + braq. |
|---|---|---|---|---|
| AD1 | [30, 31, 32, 33, 34, 35, 36] | [31, 32, 33, 34, 35, 36] | [1, 2, 3] | [1, 2, 3] |
| AD2 | [29, 30, 31, 32, 33, 34, 35, 36] | [30, 31, 32, 33, 34, 35, 36] | [1, 2, 3] | [1, 2, 3] |
| AD3 | [28, 29, 30, 31, 32, 33, 34, 35, 36] | [30, 31, 32, 33, 34, 35, 36] | [1, 2, 3] | [1, 2, 3] |
| AD4 | [28, 29, 30, 31, 32, 33, 34, 35, 36] | [30, 31, 32, 33, 34, 35, 36] | [1, 2, 3, 4] | [1, 2, 3, 4] |
| AD5 | [28, 29, 30, 31, 32, 33, 34, 35, 36] | [29, 30, 31, 32, 33, 34, 35, 36] | [1, 2, 3, 4] | [1, 2, 3, 4] |
| AD6 | [28, 29, 30, 31, 32, 33, 34, 35, 36] | [29, 30, 31, 32, 33, 34, 35, 36] | [1, 2, 3, 4, 5] | [1, 2, 3, 4, 5] |

| Textura | Sorgo 2ª |
|---|---|
| arenosa | [1, 2, 3, 4, 5] |
| media | [1, 2, 3, 4, 5, 6] |
| argilosa | [1, 2, 3, 4, 5, 6, 7, 8] |
