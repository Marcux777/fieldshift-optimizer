**DESFECHO: NEGATIVO** — NEGATIVO: P2 (minimax regret) vs P1b (perda esperada) no teste 2010/11–2024/25; mediana D = +0.0064, IC 95% [-0.0609, +0.0375], M1 0.1744 vs 0.1466. P2 vs P1 (clima médio): NEGATIVO.

Protocolo v1.1 (SHA-256 ee8e51c5b2eecc7319818a5645ef15b84d677eabab35caad1a7e9fc41deaefb9); config do protocolo 564e613d2129ffe7b65bb8bbab9261deca9e042634556b135313c41353a2896c.
Pesos (exposição, cobertura, diversidade) = [0.5, 0.25, 0.25]. |X| = 576. P0: viavel. M0 = 1.00.
Construção: 28 pares; teste: 14 pares; excluídos: 0.

| Política | Plano | M1 regret máx | M2 média | M2 mediana | M3 L seco | M3 R seco | M4a R≤ε | M4b IQR L | M4c LOO Hamming | L(s̄) |
|---|---|---|---|---|---|---|---|---|---|---|
| P0 | [0, 4, 2, 0, 4, 2] | 0.3196 | 0.2766 | 0.2819 | 0.4039 | 0.2819 | 0.00 | 0.0566 | — | 0.3015 |
| P1 | [0, 3, 3, 1, 9, 4] | 0.2811 | 0.0588 | 0.0227 | 0.1716 | 0.0496 | 0.43 | 0.0948 | 0.00 | 0.0879 |
| P1b | [0, 5, 3, 1, 7, 4] | 0.1466 | 0.0686 | 0.0766 | 0.1889 | 0.0670 | 0.21 | 0.0920 | 1.07 | 0.1002 |
| P2 | [0, 4, 3, 1, 7, 4] | 0.1744 | 0.0717 | 0.0511 | 0.1984 | 0.0764 | 0.00 | 0.0685 | 0.00 | 0.0959 |

M3: 12 pares secos no teste (P25 out–mar da construção = 1098.9 mm).
M5 preço da robustez L(P2, s̄) − L(P1, s̄) = 0.0080.

| Comparação | Idênticos | V/E/D (ε) | Mediana D | IC 95% mediana | Média D | IC 95% média | Wilcoxon 7 pares (p) | Classe | Motivo |
|---|---|---|---|---|---|---|---|---|---|
| P2 vs P1b | False | 4/4/6 | 0.0064 | [-0.0609, 0.0375] | 0.0031 | [-0.0344, 0.0368] | 0.6875 (n≠0=7) | **NEGATIVO** | M1(P2) ≥ M1 do comparador |
| P2 vs P1 | False | 2/4/8 | 0.0228 | [0.0112, 0.0549] | 0.0129 | [-0.0234, 0.0466] | 0.2969 (n≠0=7) | **NEGATIVO** | IC 95% inteiramente > 0 |

Sensibilidade (grade de 15 pesos; só relatada, não muda o veredito em w*):

| # | Pesos | P2 vs P1b | P2 vs P1 | M1 P1 / P1b / P2 |
|---|---|---|---|---|
| 1 | [1.0, 0.0, 0.0] | NEGATIVO | NEGATIVO | 0.2583 / 0.2617 / 0.2718 |
| 2 | [0.75, 0.25, 0.0] | INCONCLUSIVO | INCONCLUSIVO | 0.2508 / 0.2417 / 0.2151 |
| 3 | [0.75, 0.0, 0.25] | INCONCLUSIVO | INCONCLUSIVO | 0.1550 / 0.1812 / 0.1422 |
| 4 | [0.5, 0.5, 0.0] | NEGATIVO | INCONCLUSIVO | 0.3037 / 0.2754 / 0.2794 |
| 5 (padrão) | [0.5, 0.25, 0.25] | NEGATIVO | NEGATIVO | 0.2811 / 0.1466 / 0.1744 |
| 6 | [0.5, 0.0, 0.5] | INCONCLUSIVO | INCONCLUSIVO | 0.1004 / 0.1178 / 0.0948 |
| 7 | [0.25, 0.75, 0.0] | NEGATIVO | INCONCLUSIVO | 0.3731 / 0.3572 / 0.3660 |
| 8 | [0.25, 0.5, 0.25] | NEGATIVO | NEGATIVO | 0.3915 / 0.2350 / 0.2447 |
| 9 | [0.25, 0.25, 0.5] | NEGATIVO | NEGATIVO | 0.2242 / 0.1320 / 0.1397 |
| 10 | [0.25, 0.0, 0.75] | INCONCLUSIVO | INCONCLUSIVO | 0.0502 / 0.0589 / 0.0474 |
| 11 | [0.0, 1.0, 0.0] | INCONCLUSIVO | INCONCLUSIVO | 0.4801 / 0.4390 / 0.3247 |
| 12 | [0.0, 0.75, 0.25] | NEGATIVO | NEGATIVO | 0.5019 / 0.3375 / 0.3750 |
| 13 | [0.0, 0.5, 0.5] | NEGATIVO | NEGATIVO | 0.3346 / 0.2250 / 0.2500 |
| 14 | [0.0, 0.25, 0.75] | NEGATIVO | NEGATIVO | 0.1673 / 0.1125 / 0.1250 |
| 15 | [0.0, 0.0, 1.0] | INCONCLUSIVO | INCONCLUSIVO | 0.0000 / 0.0000 / 0.0000 |

Auditoria: {"optimizer_params_hash": "844b5ad619f2a09c211ec0c59d0fe28d82bd848e704e9d579501b8e13039f281", "params_hash_match": true, "reference_selection": {"P1": [0, 3, 3, 1, 9, 4], "P1b": [0, 5, 3, 1, 7, 4], "P2": [0, 4, 3, 1, 7, 4]}, "optimizer_matches_reference": {"P1": true, "P1b": true, "P2": true}, "x_hash_match": true, "sbar_hash_match": true, "optimizer_status": "OK"}

Limitações:
- Janelas ZARC calibradas com anos que se sobrepõem ao teste: restrição fixa e igual para todas as políticas (vazamento residual declarado, não removível).
- PRECTOTCORR pode ter correção de viés baseada em climatologia longa (vazamento residual declarado, não verificado).
- Uma célula NASA POWER (-18,0/-50,625); célula vizinha não avaliada nesta execução.
- Indicadores de exposição e de calendário; não medem produtividade, lucro, água nem solo.
- M4c (leave-one-pair-out) mantém fixas as medianas de e3 da construção completa (simplificação declarada).
- Bootstrap com 7 blocos e Wilcoxon com 7 pares: poder estatístico baixo.
