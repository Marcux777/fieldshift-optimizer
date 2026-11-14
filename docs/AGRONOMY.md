# Regras agronômicas do FieldShift (Rio Verde, GO, sequeiro)

Methods Scientist, 14/11/2026. Configuração executável: `config/catalog.toml`, `config/soil.toml`, `config/indicators.toml`. Verificação manual: `config/zarc/VERIFICATION.md`. Fontes com URL: `docs/DATA_SOURCES.md`. Marcas: **[V]** verificado; **[N]** hipótese.

## 1. Calendário
- Ano agrícola de 1/set a 31/ago, 365 dias (29/fev descartado). Horizonte de 2 anos, janelas J1/J2 no ano 1 e J3/J4 no ano 2.
- **ZARC a 20% como restrição fixa** (decisão 1): semeadura só em decêndio com classe exatamente 20% na tábua 2026/27 de Rio Verde, para a cultura, o Grupo I e o solo informado [V tábua]. O limite não sobe. A mesma tábua é aplicada a todos os anos históricos [N: premissa].
- **Soja**: semeadura legal em GO de 25/set a 02/jan e vazio de 27/jun a 24/set [V IN Agrodefesa 6/2024]. O decêndio inteiro precisa caber na janela legal, o que exclui dec27 e dec1 [N: regra conservadora]. Soja só em J1/J3.
- **2ª safra (J2/J4)**: semeadura derivada, no 1º dia ≥ colheita de J1 cujo decêndio esteja a 20% para a cultura. Sem sobreposição. **Pousio só quando nenhuma 2ª safra é viável** (decisão do Development Lead em DECISIONS.md).
- Datas: emergência = semeadura + 7 d [N; o ZARC usa 5–10 d, V]; colheita = emergência + ciclo [N]; margem colheita→semeadura = 0 d [N].

## 2. Catálogo (fechado) e grupo de ciclo
Um grupo por cultura, **Grupo I** em todas. Justificativa nas tabelas oficiais de Rio Verde [V tábua 2026/27]:

| Cultura | Janela | Ciclo (d) | Por que Grupo I |
|---|---|---|---|
| Soja | J1/J3 | 100 [N: o ZARC simula 100/115/130 d; GI = 100 d é inferência] | Grupos I–III têm decêndios idênticos a 20% em todas as AD; o mais curto deixa mais espaço para a 2ª safra |
| Milho 1ª safra | J1/J3 | 100, fases 15/35/30/20 [V Port. 178/2025] | Mesmos decêndios do Grupo II; ciclo mais curto |
| Milho 2ª safra | J2/J4 | 110 (teto do grupo) [V Port. 386/2025] | O Grupo II só tem dec1 em AD1–AD2, antes de qualquer colheita de J1 |
| Milho 2ª + braquiária | J2/J4 | 115 (teto) [V Port. 400/2025] | Mesma janela do milho 2ª GI |
| Sorgo granífero 2ª safra | J2/J4 | 110 (teto) [V Port. 123/2025] | Janela mais longa: até 20/fev (arenosa), 28/fev (média), 20/mar (argilosa) |
| Pousio | J2/J4 | — | Só quando nenhuma 2ª safra é viável |

Fases de soja, milho 2ª, consórcio e sorgo: frações 15/35/30/20% do milho 1ª GI aplicadas por proporção [N]. Milheto, crotalária, feijão e girassol estão fora (sem tabela ZARC em Rio Verde ou fora do escopo).

Contexto agronômico [V Embrapa Comunicado Técnico 377]: o sorgo é preferido ao milho em semeaduras tardias pela maior tolerância ao déficit hídrico e precocidade; a braquiária do consórcio mantém cobertura na seca e é dessecada antes da cultura seguinte. Isso não entra no motor como número.

## 3. Solo
Parâmetro informado: classe AD1–AD6 (soja e milhos) e textura (sorgo), cada uma escolhe a linha do ZARC. Padrão da demonstração **AD4 + argilosa** [N]. O app não infere o solo da fazenda nem converte textura em AD.

## 4. Inviabilidades a 20% no solo padrão (detalhe em `config/zarc/VERIFICATION.md`)
- Soja → milho 2ª (ou consórcio): só com soja em dec28–dec30 (1/out–31/out).
- Milho 1ª → milho 2ª (ou consórcio): só com milho 1ª em dec30.
- Sorgo 2ª: viável após J1 até dec34 (1–10/dez).
- J1 em dec35–dec36: nenhuma 2ª safra; J2 = pousio.
- Em AD1: soja → milho 2ª é inviável em qualquer decêndio, e o plano atual padrão (soja em 11/out) também, porque esse decêndio está a 30%.

## 5. Indicadores (só chuva e temperatura do NASA POWER)
Três eixos, todos em [0,1], menor = melhor (fórmulas exatas em `config/indicators.toml`):
- **g1 Exposição climática**: média, sobre as culturas de grão do plano, de (e1 falha de estabelecimento + e2 veranico na fase crítica + e3 déficit de chuva na fase crítica vs mediana da construção + e4 calor/geada + e5 chuva na colheita)/5.
- **g2 Cobertura vegetal**: ½(1 − fração de dias cobertos) + ½(maior intervalo descoberto / 730). Proxy de calendário.
- **g3 Diversidade**: ½(4 − espécies distintas)/3 + ½(repetições)/3.
Limiares verificados: soja Tmax > 40 °C e Tmin ≤ 3 °C; milho Tmin ≤ 2 °C; sorgo Tmax > 38 °C e Tmin < 2 °C; ≥ 6 dias de chuva no decêndio final (milho). Hipóteses: 20 mm em 10 d, 1 mm de dia seco/chuvoso, pesos iguais, 15 d de dessecação.

## 6. O que não se pode afirmar
Produtividade, lucro, economia de água, melhora ou recuperação do solo, carbono ou supressão de pragas; validade para outra fazenda, cultivo irrigado ou clima futuro; conformidade com o ZARC para seguro ou crédito (o usuário deve consultar o Painel ZARC oficial); superioridade geral do minimax regret; novidade científica.
