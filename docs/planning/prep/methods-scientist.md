# FieldShift — preparação metodológica (Methods Scientist, 13/11/2026)

## Revisão 13/11 noite (vale sobre o restante do arquivo)

Autoridade: `/home/marcusvinicus/p/fieldshift-optimizer/docs/DECISIONS.md`. Onde o texto da tarde (abaixo) conflitar com esta seção, vale esta seção.
Base: recorte oficial de Rio Verde (IBGE 5218805) da Tábua de Risco ZARC **2026/27**, em `prep/zarc-rio-verde/zarc-tabua-rio-verde-5218805-2026-2027.csv`. A tabela de 2026/27 é usada como modelo de calendário para todos os anos históricos [V: tabela; N: aplicar a mesma tabela a 1981–2025 é premissa declarada]. A viabilidade abaixo foi calculada hoje com um script descartável em `/tmp`, fora do repositório, a partir desse CSV.

### R1. Restrição ZARC
- Só são válidas semeaduras em decêndio com classe **exatamente 20%** na tábua de Rio Verde para a cultura, o grupo e o solo escolhidos. Decêndios com 30%, 40% ou não indicados ficam proibidos. O limite não sobe automaticamente [V: decisão 1].
- Soja: a janela legal de GO vai até 02/jan [V, S6]. O dec1 (1–10/jan) está a 20% no ZARC, mas passa da data legal, por isso fica **excluído** (decisão conservadora). O dec27 (21–30/set) está a 40%, então também fica excluído. Com isso, a dúvida sobre o início em 20/set (IN 5/2026) deixa de importar.
- Convenção de datas [N: hipótese de modelagem]:
  - J1/J3: semeadura no 1º dia do decêndio.
  - Emergência: 7 dias após a semeadura (o ZARC usa de 5 a 10 dias [V, S2]).
  - Colheita: semeadura + 7 + ciclo.
  - J2/J4: semeadura em max(colheita de J1, início do decêndio), válida só se cair até o fim de um decêndio a 20%.

### R2. Um grupo de ciclo por cultura (tábua 2026/27 de Rio Verde)
| Cultura (janela) | Grupo | Ciclo usado | Justificativa nas tabelas oficiais |
|---|---|---|---|
| Soja (J1, J3) | **Grupo I** | 100 d [N: o ZARC simula ciclos de 100, 115 e 130 d (S2), mas a ligação Grupo I = 100 d é inferência] | Os Grupos I, II e III têm decêndios idênticos a 20% em todas as classes AD [V]. O Grupo I é o mais curto, o único que maximiza a chance de 2ª safra. |
| Milho 1ª safra (J1, J3) | **Grupo I** | 100 d; fases 15/35/30/20 d [V, Port. 178/2025, tabela de ciclos] | Mesmos decêndios do Grupo II a 20% [V]. O Grupo III perde o dec36 em AD1. O ciclo mais curto permite a 2ª safra. |
| Milho 2ª safra (J2, J4) | **Grupo I** | ≤110 d da emergência à maturação [V, Port. 386/2025] | O Grupo I vai até dec3–dec5, conforme o AD. O Grupo II fica só no dec1 em AD1–AD2 (até 10/jan), antes de qualquer colheita de J1, logo inviável [V]. |
| Milho 2ª + braquiária (J2, J4) | **Grupo I** | ≤115 d [V, Port. 400/2025] | Janela idêntica à do milho 2ª Grupo I [V]. O Grupo II tem a mesma limitação descrita acima. |
| Sorgo granífero 2ª safra (J2, J4) | **Grupo I** | ≤110 d da emergência [V, Port. 123/2025] | É a janela mais longa: até 20/fev (arenoso), 28/fev (médio) e 20/mar (argiloso) [V]. |
| Pousio (J2, J4) | — | — | Opção nula. Também é o resultado obrigatório quando não há 2ª safra viável. |

**Saem do catálogo do MVP:** milheto e crotalária. Não há tabela ZARC para essas culturas em Rio Verde [V: ausentes no recorte], e por isso não cumprem a decisão 1. Feijão e girassol já estavam fora. O catálogo encolheu; nada foi acrescentado.

### R3. Classe de solo: parâmetro informado, com hipótese explícita
- O usuário informa o solo em duas formas: a classe **AD1–AD6**, usada por soja e milho em 2026/27, e a **textura** (arenosa, média ou argilosa), usada pelo sorgo [V: dicionário S1].
- **Padrão da demonstração: AD4 + textura argilosa.** É [N]: hipótese, não medição. Apoio indireto: o SoilGrids indica 29–51% de argila em dois pontos rurais do município [V, consultado 13/11]. A correspondência entre AD4 e "argiloso" não foi verificada; a conversão oficial usa a função de pedotransferência da IN SPA/MAPA 1/2022 [N: não lida].
- O app deve exibir o solo usado e a tabela de viabilidade do item R5 para a classe escolhida.

### R4. Segunda safra: regra de calendário do catálogo
- J2 e J4 são **opcionais**. O pousio é sempre permitido.
- Sem sobreposição: a semeadura de J2 tem de ser ≥ à colheita de J1 (R1). A colheita de J2 tem de ser anterior à semeadura de J3.
  - Pior caso: sorgo semeado em 20/mar + 7 + 110 d dá colheita em ~15/jul, antes de 1/out.
  - Esse caso sempre se cumpre no calendário atual, mas deve ficar como asserção.
- A braquiária do consórcio não é cultura nova. É continuação do consórcio de J2 até a dessecação antes de J3, e entra só no indicador de cobertura.
- Soja só em J1/J3 [V: calendário legal e vazio de 27/jun a 24/set, S6].

### R5. Combinações inviáveis a 20% (calculadas na tábua 2026/27 com os grupos de R2)
Decêndios de J1 que permitem cada 2ª safra:

| J1 → J2 | AD1 | AD2 | AD3 | AD4 (padrão) | AD5 | AD6 |
|---|---|---|---|---|---|---|
| Soja GI → milho 2ª GI (ou + braquiária) | **inviável** | dec29 | dec28–29 | dec28–30 | dec28–30 | dec28–31 |
| Milho 1ª GI → milho 2ª GI (ou + braquiária) | **inviável** | **inviável** | **inviável** | dec30 | dec29–30 | dec29–31 |

| J1 → sorgo 2ª GI (por textura) | arenoso | médio | argiloso (padrão) |
|---|---|---|---|
| Soja GI (AD3–AD6) | dec28–31 | dec28–32 | dec28–34 |
| Milho 1ª GI (AD4) | dec30–31 | dec30–32 | dec30–34 |

Também inviáveis a 20%, sem relaxar o limite:
- Qualquer J1 semeado depois do último decêndio indicado nas tabelas acima: J2/J4 só pode ser pousio.
- Milho 2ª GII e milho + braquiária GII em AD1–AD2: a janela fica só no dec1, o que é inviável após qualquer J1.
- Soja no dec1 e no dec27 (ver R1).
- Milheto e crotalária em qualquer janela: sem tabela ZARC.

Legenda dos decêndios: dec28 = 1–10/out; 29 = 11–20/out; 30 = 21–31/out; 31 = 1–10/nov; 32 = 11–20/nov; 33 = 21–30/nov; 34 = 1–10/dez.
Ressalva [N]: os limites dependem do ciclo de 100 d da soja e da emergência em 7 d. O Optimization deve testar ±7 d como sensibilidade sem mudar o padrão.

### R6. Indicadores finais: só `PRECTOTCORR`, `T2M_MAX` e `T2M_MIN`, sem radiação e sem ET0
- Ficam fora: E3 da versão anterior (razão P/ET0 de Hargreaves) e E6 (balanço hídrico com ETc). Ambos dependem de ET0 e Kc, que não temos de forma verificável.
- Todos os indicadores abaixo podem ser calculados em 1981–1983, que **permanecem** [V: série completa sem −999, segundo o MLOps e o DECISIONS.md].
- Fases do ciclo: frações 15/35/30/20% do ciclo (estabelecimento, vegetativo, floração e enchimento, maturação). Origem: tabela do milho 1ª Grupo I [V, Port. 178/2025]. Aplicar à soja, ao milho 2ª e ao sorgo é [N] hipótese. A fase crítica vai de 50% a 80% do ciclo.
- Dado ausente (−999 ou falta de dado) em qualquer dia da janela usada: o indicador vira NA, e o cenário sai **para todas as políticas** e é registrado.

**Eixo 1 — Exposição climática** (por cultura semeada; soma nas 4 janelas; pousio = 0):
- E1 Falha de estabelecimento: 1 se a chuva acumulada nos 10 d após a semeadura for < 20 mm [N: limiar].
- E2 Veranico crítico: maior sequência de dias com P < 1 mm na fase crítica [N: limiar de 1 mm].
- E3 Déficit relativo de chuva na fase crítica: max(0, 1 − P_crit / mediana de P_crit nos **anos de construção**, para a mesma cultura e decêndio) [N]. A mediana vem só da construção, para não haver vazamento.
- E4 Estresse térmico: dias na fase crítica com T2M_MAX > 40 °C para a soja [V, S2] ou > 38 °C para o sorgo [V, Port. 123], mais dias no ciclo com T2M_MIN ≤ 2 °C, critério de geada do ZARC para milho e sorgo [V, Port. 386/123]. Milho sem limiar de calor verificado, então só conta geada.
- E5 Chuva na colheita: 1 se houver ≥ 6 dias com P ≥ 1 mm no último decêndio do ciclo [V: critério do milho na Port. 386; N: limiar de 1 mm e uso na soja e no sorgo].

**Eixo 2 — Cobertura vegetal** (proxy de calendário; o clima entra pelo E1):
- C1 Fração dos 730 d com cobertura viva, da emergência à colheita. Uma cultura com E1 = 1 conta 0 dias. A braquiária conta da emergência do consórcio até 15 d antes da semeadura de J3 [N: margem de dessecação].
- C2 Maior intervalo sem cobertura viva, em dias.

**Eixo 3 — Diversidade** (só do plano, sem clima):
- D1 (ex-C3) Número de espécies distintas em J1–J4: soja, milho, sorgo e braquiária. Milho 1ª e 2ª contam como a mesma espécie. Pousio não conta.
- D2 (ex-C4) Repetições: mesma espécie na mesma janela nos dois anos (J1→J3, J2→J4), mais milho seguido de milho (J1 milho 1ª → J2 milho 2ª). É indicador, não proibição: não há fonte verificada para proibir.

O mapeamento para os três objetivos dos pesos 0,50/0,25/0,25 (exposição, cobertura, diversidade) e as normalizações fixas cabem ao Optimization e ao Evaluation. Daqui: mais E é pior, mais C1 é melhor, mais C2 é pior, mais D1 é melhor, mais D2 é pior.

### R7. Plano atual (P0) sugerido para o Evaluation
Soja GI semeada no dec29 (11/out), colheita em ~26/jan, seguida de milho 2ª GI semeado em 26/jan (dec3, a 20% em AD4), nos dois anos. É [N]: representa o sistema regional soja + milho safrinha, mas não é dado de uma fazenda.

### Mudou desde a tarde
- O limite de 30% como sensibilidade foi removido.
- Grupos II e III saíram.
- Milheto e crotalária saíram.
- E3 com ET0 e E6 foram removidos.
- C3 e C4 viraram D1 e D2.
- Solo padrão: AD4/argiloso, no lugar de AD5.

---

Somente preparação. Não há código do projeto. As marcas são: **[V]** = verificado hoje na fonte citada; **[N]** = não verificado (hipótese, valor herdado ou fonte não lida por inteiro).
Evidência bruta: `prep/zarc-rio-verde/` contém o recorte de Rio Verde (IBGE 5218805) da Tábua de Risco ZARC 2025/26 e 2026/27, o dicionário de dados, as somas SHA-256 e as URLs (`FONTE.txt`).

## Fontes principais
- S1. Tábua de Risco ZARC, dados abertos MAPA, CC-BY: https://dados.agricultura.gov.br/dataset/tabua-de-risco-zoneamento-agricola-de-risco-climatico (CSVs 2025/26 e 2026/27; dicionário em `prep/zarc-rio-verde/`). Atualizado em 13/11/2026.
- S2. Portaria SPA/MAPA nº 368/2025, ZARC da soja em GO 2025/26 (parâmetros do modelo): https://www.gov.br/agricultura/pt-br/assuntos/riscos-seguro/programa-nacional-de-zoneamento-agricola-de-risco-climatico/portarias/2025-2026/goias-go/PORTN368SOJAGO.ret2.pdf
- S3. Portaria SPA/MAPA nº 386/2025, ZARC do milho 2ª safra em GO 2025/26: https://www.gov.br/agricultura/pt-br/assuntos/riscos-seguro/programa-nacional-de-zoneamento-agricola-de-risco-climatico/portarias/2025-2026/goias-go/copy_of_PORTN33.PDF
- S4. Portaria nº 400/2025 (milho 2ª safra + braquiária): https://www.gov.br/agricultura/pt-br/assuntos/riscos-seguro/programa-nacional-de-zoneamento-agricola-de-risco-climatico/portarias/2025-2026/goias-go/PO91CB1.PDF · Portaria nº 123/2025 (sorgo granífero): …/2025-2026/goias-go/PORTN123SORGOGRANIFEROGO.pdf · Portaria nº 241/2025 (feijão 2ª safra): …/2025-2026/goias-go/PORTN23.PDF
- S5. Portaria nº 331/2022, milho 2ª safra em GO 2022/23 (CAD por textura, versão antiga): https://www.gov.br/agricultura/pt-br/assuntos/riscos-seguro/programa-nacional-de-zoneamento-agricola-de-risco-climatico/portarias/2022-2023/goias-go/port-no-331-milho-2a-safra-go-retif.pdf
- S6. Agrodefesa, vazio sanitário e calendário da soja (IN Agrodefesa nº 06/2024): https://goias.gov.br/agrodefesa/vazio-sanitario-da-soja-comeca-no-dia-27-de-junho-em-goias-informa-a-agrodefesa/
- S7. Embrapa Algodão, Comunicado Técnico 377 (2016), *Sistemas de cultivo de plantas de cobertura…*: https://www.infoteca.cnptia.embrapa.br/infoteca/bitstream/doc/1066067/1/Sistemasdecultivodeplantasdecobertura.pdf
- S8. Embrapa Milho e Sorgo, *Milheto – Plantio*: https://www.infoteca.cnptia.embrapa.br/infoteca/bitstream/doc/748815/1/Milheto-Plantio.pdf
- S9. Embrapa Soja, FBN/inoculação: https://www.infoteca.cnptia.embrapa.br/infoteca/bitstream/doc/447053/1/CNPSOCOM.TEC.7400.pdf
- S10. ISRIC SoilGrids 2.0 REST: https://rest.isric.org/soilgrids/v2.0/docs · S11. Embrapa, *Solos do município de Rio Verde (GO)*: https://www.alice.cnptia.embrapa.br/alice/bitstream/doc/553727/1/SOLOS-DO-MUNICIPIO-DE-RIO-VERDE-GO.pdf
- S12. NASA POWER Daily API: https://power.larc.nasa.gov/api/temporal/daily/point

---

## 1. Calendário de sequeiro em Rio Verde: 2 anos agrícolas, 4 janelas

O ZARC trabalha com decêndios (dec1 = 1–10/jan … dec36 = 21–31/dez) e classes de risco de perda de 20%, 30% e 40% [V, S2]. A janela depende da cultura, do grupo de ciclo e da classe de solo. **No ZARC 2026/27 a classe de solo é AD1–AD6 para soja, milho e feijão, e textura (arenosa, média ou argilosa) para sorgo e caupi; em 2025/26 o milho ainda usava textura** [V, S1]. Logo, o solo entra como parâmetro na consulta.

Estrutura proposta (o mesmo modelo nos dois anos; datas valem para qualquer ano histórico):

| Janela | Período | Restrições verificadas |
|---|---|---|
| J1 (verão, ano 1) | semeadura de 1/out a 31/dez | Soja com risco de 20%: dec28–36 em AD3–AD6, dec29–36 em AD2, dec30–36 em AD1; dec27 (21–30/set) só a 40% em AD3–AD6 [V, S1 2026/27; igual em 2025/26]. Milho 1ª safra com risco de 20%: dec29–36 em AD5–AD6, dec30–36 em AD2–AD4, dec31–36 em AD1 [V, S1]. Calendário legal da soja em GO: semeadura até 02/jan; vazio sanitário de 27/jun a 24/set, sem planta viva de soja [V, S6 para 2025/26]. Na safra 2026/27 o início foi antecipado para 20/set (IN Agrodefesa 5/2026) [N: só vi em notícia; a página deu 403]. |
| J2 (2ª safra, ano 1) | semeadura de 1/jan até o limite da cultura e do solo | Precisa começar depois da colheita de J1 (semeadura de J1 + ciclo). Limites de risco de 20% em 2026/27 [V, S1]: milho 2ª safra Grupo I até 31/jan (AD1–AD3), 10/fev (AD4–AD5) ou 20/fev (AD6); Grupo II até 10/jan (AD1–AD2), 20/jan (AD3–AD4) ou 31/jan (AD5–AD6). Milho + braquiária: limites iguais aos do milho 2ª. Sorgo granífero 2ª, Grupo I: até 20/fev (arenoso), 28/fev (médio) ou 20/mar (argiloso). Feijão 2ª, Grupo I: até 10/jan (AD3), 10/fev (AD4), 20/fev (AD5) ou 28/fev (AD6). Milheto de cobertura (sem ZARC): fim de jan a meados de abr [V, S8]. |
| Entressafra seca | da colheita de J2 até J3 | Sem cultura comercial zoneada. Pode ficar sob palhada ou braquiária, ou em pousio. Vazio sanitário de 27/jun a 24/set [V, S6]. |
| J3 e J4 (ano 2) | iguais a J1 e J2 | As mesmas regras, com o clima do ano 2. |

Restrições de acoplamento a implementar:
- R1 [V]: a semeadura precisa cair num decêndio com classe ≤ ao limite de risco escolhido (sugestão: 20%; 30% como análise de sensibilidade) na tábua de Rio Verde para cultura, grupo e solo.
- R2 [V, S2/S3]: ciclos dos grupos ZARC. Soja: 100, 115 ou 130 dias. Milho 2ª: Grupo I ≤110 dias, Grupo II de 110 a 140 dias, contados **da emergência à maturação**. A emergência ocorre 5–10 dias após a semeadura. Colheita = semeadura + emergência + ciclo [N: a convenção exata de contagem para a soja não está explícita em S2].
- R3: semeadura de J2 ≥ colheita de J1 + margem operacional (sugestão 0–5 dias) [N: a margem é hipótese]. Consequência: soja semeada em dezembro inviabiliza milho e sorgo de 2ª safra dentro do ZARC, deixando só cobertura ou pousio. Essa é a principal fonte de trade-off do problema.
- R4 [V, S6]: soja só em J1 e J3. A janela legal termina em 02/jan e, após 27/jun, não pode haver planta viva de soja. Soja em J2 fica fora do modelo (decisão de escopo apoiada no calendário).
- Lacuna: o ZARC foi calibrado com ≥15 anos de estações [V, S2] que se sobrepõem aos anos de teste da avaliação retrospectiva. As janelas devem ser tratadas como **restrição externa fixa e igual para as três políticas**, e essa limitação deve ser declarada.

## 2. Catálogo mínimo (6 opções + pousio)

| Opção | Janela | Água / risco hídrico | N | Cobertura do solo | Restrições de sucessão |
|---|---|---|---|---|---|
| Soja (grupo I, II ou III) | J1, J3 | 450–800 mm/ciclo; pico de 7–8 mm/dia na floração e no enchimento; ISNA ≥0,55 na fase III [V, S2] | Leguminosa; FBN com *Bradyrhizobium* dispensa N mineral [V, S9] | Pouca palha [V, S7: "soja aporta pouca matéria seca"] | Soja–soja em J1→J3 é monocultura de verão: indicador, não proibição [N] |
| Milho 1ª safra | J1, J3 | Fase crítica no florescimento e enchimento; ISNA ≥0,5 [V, S3, mesmos critérios do 2ª] | Gramínea, exige N [N: dose não levantada] | Muita palha [N] | Milho 1ª seguido de milho 2ª: evitar (pragas e doenças comuns) [N: sem fonte lida] |
| Milho 2ª safra | J2, J4 | ISNA ≥0,6 na fase I e ≥0,5 na fase III; perda se houver ≥6 dias de chuva no decêndio final [V, S3] | Exige N; sucede bem a soja [N] | Palhada após a colheita [N] | Precisa de colheita cedo em J1 (R3) |
| Milho 2ª + braquiária (*Urochloa*) | J2, J4 | ZARC próprio, com as mesmas janelas do milho 2ª [V, S1/S4] | — | Braquiária continua na seca e dá cobertura contínua até J3 [V, S4 item 1.3; S7]. *U. ruziziensis* tem pouca tolerância à seca [V, S7] | Dessecar antes da soja de J3 [V, S7] |
| Sorgo granífero 2ª safra | J2, J4 | Mais tolerante ao déficit hídrico e mais precoce que o milho, por isso preferido em semeadura tardia [V, S7]; janela ZARC mais longa (até 20/mar em solo argiloso) [V, S1]; restrição térmica: Tmin <12 °C e Tmax >38 °C [V, S4] | Exige N [N] | Rebrota após a colheita e ganha massa com as chuvas de primavera [V, S7] | — |
| Cobertura: milheto (alternativa: *Crotalaria spectabilis/ochroleuca*) | J2, J4 (milheto de fim de jan a meados de abr) | Milheto tem boa tolerância à seca [V, S7, S8]; *C. ochroleuca* tolera déficit hídrico [V, S7] | Milheto: não fixa N. Crotalárias: alta FBN [V, S7] e controle de nematoides [V, S7] | Milheto: 5–15 t MS/ha [V, S7] | Sem ZARC; janela de crotalária em Rio Verde [N] |
| Pousio | J2, J4 | — | — | Solo sem cobertura viva | Opção nula, necessária como baseline |

Opcional, fora do mínimo: feijão 2ª safra (ZARC verificado, S1) e girassol (ZARC verificado, S1). Não entram no MVP.
Parâmetros de produtividade, preço e custo **não** fazem parte do catálogo (ver item 5).

## 3. Solo

- **Recomendação:** solo informado pelo usuário como parâmetro explícito, no formato do ZARC: classe AD1–AD6 (soja e milho 2026/27) e textura arenosa/média/argilosa (sorgo). O padrão da demonstração deve ser "argiloso / AD5", marcado como **hipótese**, com análise de sensibilidade em AD3–AD6 [N: o padrão é escolha nossa].
- **CAD por classe para o balanço hídrico opcional** [V, S2]: AD1–AD6 = 24, 32, 42, 55, 72 e 95 mm (soja, raiz efetiva de 60 cm). Para o milho, versão de 2022: arenoso, médio e argiloso = 30, 47 e 72 mm, com 45 cm de raiz [V, S5].
- **Fonte identificável para contexto:** SoilGrids 2.0, global, 250 m [V, S10, consultado hoje]. Na coordenada urbana (−17.80, −50.93) a consulta retorna `null` (área mascarada). Em pontos rurais: (−17.65, −51.05) teve argila de 40% em 0–5 cm e 51% em 30–60 cm; (−17.95, −50.80) teve 29% e 40%. Isso indica textura média a argilosa variável dentro do município [V]. A conversão de granulometria para classe AD segue a função de pedotransferência da IN SPA/MAPA nº 1/2022 [V: citada em S2; N: fórmula não lida].
- Embrapa: legenda de solos de Rio Verde com predomínio de Latossolos [V, S11 é documento antigo; N: escala e ano não conferidos].
- Lacuna: não há mapa de solo por talhão. O app não deve inferir o solo da fazenda a partir de dados globais.

## 4. Indicadores (só com P e T do NASA POWER)

Entradas por dia [V, S12, testado hoje em −17.80/−50.93, fonte MERRA-2/POWER]: `PRECTOTCORR` (mm/dia), `T2M`, `T2M_MAX` e `T2M_MIN` (°C); valor ausente = −999. Cada ocorrência de cultura c, com semeadura s, ciclo L e ano climático y, gera uma linha de indicadores. As fases são frações de L definidas como parâmetros [N: as portarias não dão a duração das fases].

**Exposição a período seco** (quanto maior, pior):
- E1 Estabelecimento: P acumulada nos 10 dias após a semeadura < θ₁ (binário). θ₁ é hipótese (sugestão: 20 mm) [N].
- E2 Veranico crítico: maior sequência de dias com P < 1 mm na fase crítica (floração e enchimento; sugestão: 45–80% de L) [N: limiar de 1 mm e frações].
- E3 Déficit crítico: ΣP na fase crítica dividida por ΣET0 na mesma fase. ET0 por Hargreaves-Samani usa só Tmax, Tmin, Tmean e a radiação extraterrestre (calculada da latitude e do dia do ano) [N: equação FAO-56 nº 52 não reconferida hoje].
- E4 Calor crítico: dias com T2M_MAX acima do limiar da cultura na fase crítica. Soja >40 °C [V, S2]; feijão ≥36 °C [V, S4]; sorgo >38 °C [V, S4]; milho [N].
- E5 Chuva na colheita: ≥6 dias com chuva no decêndio final do ciclo [V, critério ZARC do milho, S3]. O limiar de "dia com chuva" é hipótese (sugestão: ≥1 mm) [N].
- E6 (opcional) Balanço hídrico em balde com a CAD da classe de solo e razão ETr/ETc na fase crítica, como proxy do ISNA. **Não reproduz o ZARC** (faltam Kc e fases) [N].
- Agregação por plano: soma ou máximo nas 4 janelas; por política: distribuição nos anos (média, pior caso e arrependimento).

**Diversidade e continuidade de cobertura** (proxies indiretos derivados do plano e do catálogo; o clima entra só via E1):
- C1 Fração de dias com cobertura viva em 730 dias. Uma cultura só conta como cobertura se E1 indicar estabelecimento. A braquiária conta até a dessecação antes de J3.
- C2 Maior intervalo sem cobertura viva, em dias.
- C3 Número de espécies distintas e de famílias (Poaceae/Fabaceae) nas 4 janelas, e presença de leguminosa fora da soja.
- C4 Repetição: número de vezes em que a mesma cultura ocupa a mesma janela nos 2 anos (soja–soja em J1→J3; milho–milho em J2→J4).
- Nenhum desses indicadores mede biomassa, carbono ou NDVI. São contagens de calendário. Validar com MODIS/Landsat fica fora do MVP.

**Regra de método:** limiares, frações de fase e pesos ficam congelados antes da avaliação retrospectiva e são escolhidos só com os anos de construção. Mudança depois de ver o teste invalida a comparação (alinhar com Evaluation).

## 5. O que NÃO pode ser afirmado

1. Produtividade (sc/ha) ou ganho de produtividade de qualquer rotação: não há modelo de cultura calibrado. "Risco ZARC 20%" significa probabilidade de perda pelo modelo do MAPA, não produção esperada [V, S2 nota 3].
2. Lucro, receita, retorno ou economia de custo, inclusive economia de N pela FBN em reais.
3. Economia de água, uso de água ou "eficiência hídrica": E3/E6 são exposições climáticas, não medições de consumo.
4. Recuperação do solo, ganho de matéria orgânica, carbono, estrutura ou supressão de nematoides: C1–C4 são proxies de calendário.
5. Que a recomendação é válida para outra fazenda ou município, para cultivo irrigado ou para o clima futuro: avaliação só retrospectiva, em um ponto POWER (célula MERRA-2 de ~0,5° × 0,625° [N: resolução não reconferida hoje]).
6. Que minimax regret é superior em geral: só nas condições testadas, com resultado negativo reportado se ocorrer.
7. Que o plano "cumpre o ZARC" no ano corrente para fins de seguro ou crédito. O app usa uma cópia datada da tábua; o usuário deve consultar o Painel ZARC oficial.
8. Novidade científica (regra do briefing).

## Lacunas e próximos passos (14/11)
- [N] IN Agrodefesa 5/2026 (início em 20/set na safra 2026/27): confirmar na fonte oficial ou usar 25/set.
- [N] Duração das fases fenológicas por cultura: adotar frações declaradas como hipótese e testar a sensibilidade.
- [N] Janela da crotalária em Rio Verde e limiar de calor do milho: sem fonte; manter crotalária como alternativa opcional.
- [N] Margem operacional entre colheita e semeadura.
- Decisão para o Dev Lead/Optimization: limite de risco ZARC (20% recomendado) e conjunto de grupos de ciclo no MVP (sugestão: soja GI/GII, milho GI).
