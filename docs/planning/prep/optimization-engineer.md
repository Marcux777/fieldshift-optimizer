# FieldShift — formulação matemática e escolha de método (Optimization Engineer, 13/11/2026)

## Revisão 13/11 noite (vale sobre o restante do arquivo)

Autoridade: `/home/marcusvinicus/p/fieldshift-optimizer/docs/DECISIONS.md`. Onde o texto da tarde (abaixo, "Texto anterior") conflitar com esta seção, vale esta seção. Somente papel: nada foi implementado nem executado, exceto uma contagem aritmética da grade (R2). Marcas: **[H]** hipótese a congelar antes da 1ª execução; **[T]** vira teste em 14/11. Termos alinhados ao protocolo de avaliação (P0, P1 = x_CONV, P1b = x_EXP, P2 = x_MMR, S_constr, S_teste, L, R, ε). Indicadores e catálogo: seção "Revisão 13/11 noite" de `prep/methods-scientist.md` (E1–E5, C1–C2, D1–D2, um grupo de ciclo GI por cultura). O protocolo do Evaluation ainda estava na v1.0 quando escrevi; os pontos que dependem dele estão em R7.

### R0. O que mudou
- **Saiu:** f₄ (ocupação da 2ª safra), limite ZARC de 30%, grupos de ciclo como variável, grade W0–W4, CP-SAT como plano B.
- **Entrou:** três objetivos com pesos 0,50/0,25/0,25; grade de 15 pesos; normalizações teóricas; desempate e dados ausentes explícitos; saída "inviável" sem relaxar.
- **Mudou de tamanho:** com um grupo de ciclo por cultura e a semeadura de J2/J4 derivada (R3), |X| cai de ~4·10⁵ para **≤ ~4·10³** (estimativa; contagem real em 14/11).

### R1. Três objetivos (todos em [0,1] por construção, "menor é melhor")

Cenário s = par de anos agrícolas (y, y+1); J1–J2 usam o clima de y, J3–J4 o de y+1.

**g₁ Exposição climática** (Eixo 1 do Methods). Para cada ocorrência de cultura de grão o_w no ano climático y:
- e₁ ∈ {0,1} (falha de estabelecimento); e₅ ∈ {0,1} (chuva na colheita).
- e₂ = maior sequência de dias com P < 1 mm na fase crítica ÷ nº de dias da fase crítica.
- e₃ = max(0, 1 − P_crit/mediana), com a mediana de P_crit por (cultura, decêndio) **só dos anos de construção**; se a mediana for 0, e₃ = 0.
- e₄ = média dos componentes aplicáveis: (dias com T2M_MAX acima do limiar na fase crítica ÷ dias da fase) e (dias com T2M_MIN ≤ 2 °C no ciclo ÷ dias do ciclo). Milho só tem geada, e não tem limiar de calor verificado.
- Exposição da ocorrência: ε = (e₁+e₂+e₃+e₄+e₅)/5 [H: pesos iguais].
- **g₁(x,s) = média de ε sobre as janelas com cultura de grão** (2 a 4 janelas; J1 e J3 sempre têm). Divido pelo número de culturas, e não por 4: uma soma ou uma divisão fixa por 4 premiaria o pousio só por ter menos culturas. O pousio não entra em g₁; seu custo aparece em g₂ e g₃. Difere do "soma nas 4 janelas" do Methods; peço alinhamento em R7.

**g₂ Cobertura vegetal.** g₂ = ½(1 − C1) + ½·C2/T, com C1 = fração dos T dias com cobertura viva (cultura com e₁ = 1 conta 0 dias; braquiária vale até 15 d antes de J3) e C2 = maior intervalo sem cobertura viva, em dias. T = 730 dias (R4, 29/fev descartado). O clima entra só por e₁.

**g₃ Diversidade.** g₃ = ½·(4 − D1)/3 + ½·D2/3, com D1 = número de espécies distintas (soja, milho, sorgo, braquiária; pousio não conta) e D2 = número de repetições (J1→J3, J2→J4, e milho 1ª → milho 2ª), cada uma entre 0 e 3. Não depende do clima.

**Normalizações fixas.** Todas teóricas: razões entre contagens de dias, binários e limites do número de espécies e de repetições. **Nenhum mínimo e máximo é calculado a partir de dados**, nem de construção nem de teste. Os únicos parâmetros vindos de dados são as medianas de e₃, calculadas só na construção e gravadas com hash. Como tudo fica em [0,1], nenhum valor de teste precisa ser cortado [T: asserção 0 ≤ g_k ≤ 1 em todo (x,s)].

**Perda escalar** (a mesma para todas as políticas): L_w(x,s) = w₁g₁ + w₂g₂ + w₃g₃, com w = (exposição, cobertura, diversidade). **Padrão w\* = (0,50; 0,25; 0,25)**. Todas as políticas e a avaliação principal usam w\*.

### R2. Grade de 15 pesos (confirmado: dá exatamente 15)
Triplas (a,b,c)/4 com a+b+c = 4, a,b,c inteiros ≥ 0. O número é C(6,2) = 15 (conferi também por contagem direta, 15). Lista fixa, em ordem decrescente de exposição, depois de cobertura:

| # | exposição | cobertura | diversidade |  | # | exposição | cobertura | diversidade |
|---|---|---|---|---|---|---|---|---|
| 1 | 1,00 | 0,00 | 0,00 | | 9 | 0,25 | 0,25 | 0,50 |
| 2 | 0,75 | 0,25 | 0,00 | | 10 | 0,25 | 0,00 | 0,75 |
| 3 | 0,75 | 0,00 | 0,25 | | 11 | 0,00 | 1,00 | 0,00 |
| 4 | 0,50 | 0,50 | 0,00 | | 12 | 0,00 | 0,75 | 0,25 |
| **5** | **0,50** | **0,25** | **0,25** (padrão) | | 13 | 0,00 | 0,50 | 0,50 |
| 6 | 0,50 | 0,00 | 0,50 | | 14 | 0,00 | 0,25 | 0,75 |
| 7 | 0,25 | 0,75 | 0,00 | | 15 | 0,00 | 0,00 | 1,00 |
| 8 | 0,25 | 0,50 | 0,25 | | | | | |

- A grade serve só à **análise de sensibilidade**: para cada w, recalculam-se x_CONV(w), x_EXP(w) e x_MMR(w) na construção, e o teste é só medido e relatado por inteiro. Nunca se usa para escolher.
- **Ressalva (sem mudar a interpretação):** 12 dos 15 pontos têm algum peso zero. Aí vários planos empatam em L e o minimizador pode ser só fracamente eficiente; o desempate de R5 resolve de forma determinística, e "planos idênticos entre políticas" é esperado nesses pontos (nos vértices 1, 11 e 15 alguns objetivos são constantes no cenário). Se o usuário preferir pesos estritamente positivos, a alternativa de 15 pontos é múltiplos de 0,1 com mínimo 0,2, mas ela **não contém** 0,50/0,25/0,25. Mantenho a grade do Development Lead.

### R3. Conjunto viável X: ZARC ≤ 20% fixo, 2ª safra opcional
- **Variável:** plano x = (o₁, c₂, o₃, c₄). J1/J3: o = (cultura ∈ {soja GI, milho 1ª GI}, decêndio de semeadura d). J2/J4: cultura c ∈ {milho 2ª GI, milho 2ª + braquiária GI, sorgo GI, pousio}. Não há grupo de ciclo como variável (um por cultura, Methods R2).
- **Restrição ZARC fixa:** só decêndios com classe **exatamente 20%** na tábua de Rio Verde 2026/27, para o solo informado. Não há parâmetro para elevar o limite, nem como sensibilidade.
- **Calendário de J1/J3:** semeadura no 1º dia do decêndio; soja fora do dec1 e do dec27 (Methods R1).
- **J2/J4, regra de calendário derivada (não é variável):** a data de semeadura é a **primeira data ≥ colheita(J1) + m dentro de um decêndio a 20%** para a cultura e o solo (m = 0 [H]; Methods usa max(colheita, início do decêndio)). Se não houver, a cultura é inviável nessa janela. Sem sobreposição: σ(J2) ≥ colheita(J1) e colheita(J2) < σ(J3), com asserção [T].
- **Pousio:** sempre permitido em J2/J4, e é a única opção quando nenhuma 2ª safra é viável. Isso é uma opção de calendário, e não um objetivo.
  - **Interpretação adotada:** "2ª safra opcional" = a 2ª safra pode ser omitida (pousio). Se a intenção era "pousio só quando nada é viável", a regra é uma linha a mais no verificador; o Development Lead diz.
- **Sucessões:** nenhuma proibição agronômica (Methods não tem fonte); só indicadores (D2).
- **Tamanho (estimativa de teto):** AD4, soja ≤ 9 decêndios e milho 1ª ≤ 7, vezes ≤ 4 opções em J2: ≤ 64 módulos por ano e ≤ 64² = **4.096 planos** [T: contar]. Não há guarda de tamanho nem solver.
- **Saída "inviável":** X = ∅ só ocorre se O₁ = ∅ (nenhum decêndio a 20% para soja ou milho 1ª no solo informado). O motor devolve `INVIAVEL` com o diagnóstico (por cultura e janela, quais decêndios foram excluídos e por quê) e **não** relaxa o risco, não troca de grupo nem muda o solo. Se algum J2/J4 não tiver cultura viável, isso **não** é inviabilidade: o pousio cobre.

### R4. Dados ausentes e calendário de dias (regra única, igual para todas as políticas)
1. Antes de qualquer conta, −999 e NaN viram **ausente**, nunca 0 mm.
2. **Dia 29/fev é descartado** de todas as séries (reais e s̄): todo ano agrícola tem 365 dias, T = 730, e os índices de dia das janelas são fixos [H; testar ±1 dia na sensibilidade].
3. **Cenário válido** = nenhum dado ausente em `PRECTOTCORR`, `T2M_MAX` e `T2M_MIN` de 1/set de y a 31/ago de y+2. A validade depende só do dado, não do plano nem da política, então todas as políticas e o oráculo usam exatamente o mesmo S_constr e S_teste. Excluir por plano criaria viés de seleção.
4. Cenários excluídos ficam num log (contagem por variável e ano), antes da 1ª execução.
5. s̄ (clima médio) é a média diária por (mês, dia) dos anos agrícolas válidos da construção. Se algum dia de s̄ ficar sem nenhum valor, P1 não é calculável e o motor aborta com erro explícito. **Sem imputação.**
6. Portões: se restarem < 20 dos 28 pares da construção, ou < 10 dos 14 do teste, a avaliação para com "dados insuficientes" [H: limiares]. O MLOps verificou 16.436 dias sem −999 em `PRECTOTCORR`/`T2M`, então em princípio nada é excluído; a regra existe para o caso de a coleta mudar. Radiação não é usada.

### R5. Arrependimento, minimax regret e desempate determinístico
- **Oráculo e regret:** m_s = min_{x′∈X} L_w(x′,s); R(x,s) = L_w(x,s) − m_s ≥ 0, com R = 0 no plano oráculo. Mesmo X e mesma L nos dois lados.
- **x_MMR** = argmin_{x∈X} max_{s∈S_constr} R(x,s).
- **Comparação numérica (um só critério, igual para todas as seleções):** q(v) = round(v, 9) sobre float64, com soma sempre na mesma ordem (janelas w = 1..4; cenários em ordem cronológica).
- **ID do plano:** a tupla de índices (o₁, c₂, o₃, c₄) na ordem do catálogo (cultura na ordem do catálogo e decêndio crescente), e comparação lexicográfica. O ID independe da ordem de enumeração.
- **Desempate lexicográfico, em ordem:**
  1. **x_MMR:** menor q(máx_s R); depois menor q(média_s R); depois menor ID.
  2. **x_CONV:** menor q(L_w(x, s̄)); depois menor q(média_s L); depois menor ID.
  3. **x_EXP:** menor q(média_s L); depois menor q(máx_s R); depois menor ID.
- **Propriedade [T]:** com o segundo critério, x_MMR é não dominado no vetor (L(·,s))_s (se y dominasse x, y teria regret máximo ≤ e regret médio estritamente menor).
- **Tolerância de empate:** a tolerância de 1e-9 é só numérica. O ε = 0,02 do protocolo continua sendo a convenção estatística da classificação do resultado; as duas não se misturam.
- **Condições equivalentes:** as quatro políticas usam o mesmo X, o mesmo snapshot, os mesmos S_constr e S_teste (após o item R4.3), a mesma w, a mesma L e as mesmas normalizações, e são avaliadas pelo mesmo avaliador, separado do otimizador. Só muda a regra de escolha.
  - **P0:** x₀ dado, ou o padrão da demo (soja GI no dec29 + milho 2ª GI em 26/jan, nos dois anos, Methods R7 [N]). O app valida x₀ contra X. **Se x₀ ∉ X** (viola ZARC ≤ 20% ou calendário), o resultado é "plano atual inviável sob ZARC ≤ 20%" e P0 **não** entra nas comparações de regret; P1, P1b e P2 seguem normalmente.
  - **P1:** x_CONV avalia L em s̄, aplicado aos dois anos, e depois é avaliado nos mesmos cenários reais que os demais.
  - **P1b:** x_EXP.
- **Pareto (continua valendo, só reduzido):** F(x) = (ḡ₁, g₁^max, ḡ₂, ḡ₃) sobre S_constr, em que g₁^max (pior exposição) é só uma estatística de g₁, não um quarto objetivo nem tem peso. Dominância com valores arredondados por q. Com |X| ≤ ~4·10³, a fronteira sai por comparação par a par exata. Alternativas distintas: assinatura = (cultura₁, c₂, cultura₃, c₄); um representante por assinatura; Hamming mínimo 2 entre as exibidas (relaxa a 1, com aviso); nunca duplicatas.

### R6. Método (sem mudança): enumeração exaustiva em numpy, sem solver
|X| ≲ 4·10³ × 42 cenários dá menos de 2·10⁵ avaliações plano×cenário: a enumeração é exata, dá o oráculo e a fronteira diretamente, e leva segundos [estimativa, não medida]. Bibliotecas mínimas: `numpy`, `pandas` (tabelas) e `pytest`, mais a biblioteca padrão (`itertools`, `dataclasses`, `datetime`, `hashlib`, `json`, `tomllib`). `scipy.stats` (Wilcoxon) fica com o Evaluation; `streamlit` e o gráfico, com o Software Engineer.

### R7. Alinhamento com o protocolo do Evaluation (pontos a confirmar na v1.1)
1. **L** com três objetivos e w\* = 0,50/0,25/0,25; a grade de 15 pesos da R2, para sensibilidade; sem W0–W4 e sem limite ZARC de 30%.
2. **Normalização:** teórica; sem mín/máx de dados. A "Normalização (mínimo e máximo de cada indicador)" da v1.0 passa a "medianas de e₃ da construção".
3. **Desempate:** os três critérios de R5, no lugar do "ID lexicográfico" puro.
4. **Ausentes:** a regra de R4, incluindo 29/fev descartado e o portão de dados insuficientes.
5. **Exposição:** g₁ é a média sobre as culturas de grão (e não a soma nas 4 janelas do Methods). Methods e Evaluation precisam concordar.
6. **P0 inviável:** o gate M0 = 100% vale para P1, P1b e P2; P0 fora de X é relatado como "inviável sob ZARC ≤ 20%" e sai da comparação.
7. **Saída "inviável" (X vazio):** é um resultado, não uma execução com falha.

### R8. Riscos de correção a testar em 14/11 (além de T1–T7 do protocolo)
1. **Reprodução do R5 do Methods:** o motor tem de reproduzir exatamente a tabela de decêndios de J1 que permitem 2ª safra (AD4: soja → milho 2ª em dec28–30). Divergência é bug ou convenção diferente (m, emergência de 7 d) [T].
2. **Contagem e viabilidade:** |X| real, descartes por restrição e asserções de não sobreposição (σ(J2) ≥ colheita(J1); colheita(J2) < σ(J3)) [T].
3. **Faixa [0,1]:** 0 ≤ g_k ≤ 1 em todo (x,s), construção e teste, sem nenhum mín/máx de dados; trocar os anos de teste não muda medianas, x_CONV, x_EXP nem x_MMR (T2 do protocolo) [T].
4. **Grade:** 15 pontos, cada um somando 1, com 0,50/0,25/0,25 presente e sem duplicatas [T].
5. **Determinismo do desempate:** embaralhar a ordem de enumeração e a ordem dos cenários não muda x_MMR, x_CONV nem x_EXP; instância de brinquedo com empates exatos, feita à mão [T].
6. **Ausentes:** injetar −999 em um dia exclui o cenário para todas as políticas, não vira 0 mm, não entra em s̄; um dia de s̄ sem valor aborta; 29/fev descartado em todas as séries (365 dias) [T].
7. **Inviável:** solo/cultura sem decêndio a 20% em J1 devolve `INVIAVEL` com diagnóstico, e o limite não sobe; J2 sem cultura viável vira pousio sem erro [T].
8. **P0:** plano da demo pertence a X; plano injetado com semeadura a 30% é recusado e sai da comparação [T].
9. **Incentivo do pousio:** verificar, em instância de brinquedo, que g₁ como média das culturas não premia o pousio só por ter menos culturas, e relatar quantas políticas escolhem pousio em J2/J4 [T].
10. **Avaliador duplo:** avaliador vetorizado × avaliador ingênuo em laços, em todo X (|X| é pequeno), igual até 10⁻⁹ [T].
11. **MMR:** invariâncias a constante por cenário, a escala positiva e à permutação de cenários; x_MMR não dominado em (L(·,s))_s; R ≥ 0 e R = 0 no oráculo [T].
12. **Pareto:** fronteira conferida por comparação exaustiva; nenhum elemento fora dela é não dominado [T].
13. **Degeneração:** se g₁ variar pouco entre planos, as políticas coincidem. Relatar como resultado, sem ajustar pesos [H].
14. **Sensibilidade a ±7 d** de emergência/ciclo (pedido do Methods): só mede quanto |X| e a recomendação mudam; não altera o padrão nem entra na seleção [T].

---

# Texto anterior (tarde de 13/11) — superado onde conflitar com a revisão acima


Somente papel: nada foi implementado nem executado. Marcas: **[H]** = hipótese a congelar antes da 1ª execução; **[T]** = vira teste em 14/11. Os parâmetros do catálogo vêm de `prep/methods-scientist.md`; o protocolo de avaliação vem de `prep/evaluation-protocol.md` (v1.0). Os tamanhos de espaço e de tempo abaixo são estimativas de ordem de grandeza, não medições.

## 1. Variáveis de decisão e restrições

**Índices e dados (todos parâmetros do catálogo).**
- Janela w ∈ {1,2,3,4}. J1 e J3 são de verão; J2 e J4 são de 2ª safra. O ano agrícola é a(w) = 1 para w ∈ {1,2} e a(w) = 2 para w ∈ {3,4}. Cada ano vai de 1/set a 31/ago.
- Opção o = (cultura c, grupo de ciclo g, decêndio de semeadura d), ou "sem semeadura" (pousio). Cada opção tem:
  - data de semeadura σ(o) e duração de emergência à maturação, que dão a data de colheita h(o) = σ(o) + emergência + ciclo (valores do ZARC e do Methods);
  - classe de risco ZARC ρ(o, solo), com o solo como parâmetro (AD1–AD6 ou textura);
  - janelas permitidas W_c e, para cobertura, a data de fim da cobertura viva (braquiária: dessecação antes de J3).
- Parâmetros globais: ρ_max (20%), margem operacional m (0–5 dias), matriz de sucessões proibidas B[c,c′], mínimo de culturas de grão por ano.

**Variável.** x = (o₁,o₂,o₃,o₄), com o_w ∈ O_w. Na forma binária, z_{w,o} ∈ {0,1} e Σ_o z_{w,o} = 1 para cada w. O conjunto de opções O_w já traz as restrições de pertinência (K0).

**Restrições obrigatórias (verificador determinístico; plano inviável não conta).**
- **K0, pertinência e calendário.** w ∈ W_c(o); ρ(o,solo) ≤ ρ_max; σ(o) cai dentro da janela legal.
  - Soja só em J1 e J3; semeadura até 02/jan; nada de soja viva entre 27/jun e 24/set; início da semeadura em 20/set ou 25/set [H, o Methods marca como não verificado].
  - Milho 1ª só em J1/J3. Milho 2ª, milho + braquiária, sorgo e cobertura só em J2/J4.
- **K1, sucessão no mesmo ano.** σ(o₂) ≥ h(o₁) + m e σ(o₄) ≥ h(o₃) + m. Pousio não tem semeadura e não impõe nada.
- **K2, sucessão entre anos.** h(o₂) + m ≤ σ(o₃) (com a dessecação da braquiária). Cada ocupação termina dentro do próprio ano agrícola: h(o_w) ≤ 31/ago do ano a(w).
- **K3, sucessões proibidas.** B[c(o_w), c(o_{w+1})] = 0 para w = 1,2,3. Padrão: B vazia, salvo o que o Methods citar com fonte (milho 1ª → milho 2ª está como [N] lá, então fica como parâmetro desligado). Soja–soja e milho–milho entre anos são indicadores (item 2), não restrições.
- **K4, uso mínimo.** No mínimo 1 cultura de grão por ano. É satisfeita por construção, porque J1 e J3 só têm soja ou milho 1ª; mesmo assim vale como asserção.

**Propriedade-chave.** Se a fenologia é fixa (ciclo em dias, [H]), K0–K4 dependem só de datas, não do clima. Logo o conjunto viável X é **o mesmo em todos os cenários**, e a enumeração é feita uma única vez. Se o ciclo passar a depender do clima, esta propriedade cai e é preciso refazer o desenho.

**Tamanho.** Se |O₁|=|O₃| ≤ ~30 e |O₂|=|O₄| ≤ ~20, então |X| ≤ 360.000 antes de K1–K3. Com 42 cenários isso dá ~15 milhões de pares plano×cenário, o que cabe em memória se for processado em blocos [T: contar |X| real].
- **Guarda de tamanho.** Se |X|·|S| > ~5·10⁷, limitar a data de semeadura por (cultura, grupo) a 3 valores (primeiro, central e último decêndio com risco ≤ ρ_max).

## 2. Objetivos candidatos (todos em "menor é melhor", só POWER e calendário)

**Cenário** s = par de anos agrícolas (y, y+1), como no protocolo. As janelas J1–J2 usam o clima do ano y; J3–J4, o do ano y+1.

**Indicadores brutos** (definições do Methods): E1–E5 por ocorrência de cultura e ano; C1–C4 por plano. Orientação "maior = pior": para E3 uso e₃ = 1 − min(1, ΣP/ΣET0).

**Normalização.** ẽ_j = (e_j − mín_j)/(máx_j − mín_j), com mín e máx tomados sobre todos os pares (opção, ano) da construção. A mesma normalização vale para todos os cenários e todas as políticas; os valores do teste não são cortados. Os pesos a_j são iguais e comuns a todas as culturas; um indicador não aplicável contribui com 0 (cobertura e pousio só têm E1).

- **f₁ Exposição a seca.** f₁(x,s) = ¼ Σ_w Σ_j a_j ẽ_j(o_w, y(w,s)).
- **f₂ Descontinuidade de cobertura.** f₂ = ½(1 − C1) + ½ C2/T, com T = 730 ou 731 dias. A cobertura só conta se E1 indicar estabelecimento, que é a única via do clima.
- **f₃ Déficit de diversidade.** f₃ = ½ · rep/2 + ½ · (4 − n_esp)/3, com rep = número de repetições da mesma cultura em (o₁,o₃) e (o₂,o₄) e n_esp = número de espécies distintas.
- **f₄ Ocupação.** f₄ = (número de janelas J2/J4 sem cultura de grão)/2. É um proxy de calendário para a prioridade de usar a janela, **não** de renda nem de produtividade.

**Perda escalar** (a mesma do protocolo): L(x,s) = Σ_k w_k f̃_k(x,s), com w ≥ 0 e Σ w = 1.
- **Peso primário W0** = (¼,¼,¼,¼), por falta de informação.
- **Grade de sensibilidade (fixar antes das 12h de 14/11):**
  - W1 = (.55,.15,.15,.15), foco em exposição;
  - W2 = (.15,.55,.15,.15), foco em cobertura;
  - W3 = (.15,.15,.55,.15), foco em diversidade;
  - W4 = (.15,.15,.15,.55), foco em ocupação.
  - A grade é simétrica e não é escolhida para favorecer nenhuma política.

**Alerta de correção (pedido de emenda ao protocolo v1.0).** O protocolo lista E1–E5 e C1–C4 mas não tem f₄. Sem f₄, e com a opção de cobertura expondo-se só a E1, o ótimo em J2/J4 tende a ser "cobertura ou pousio" por construção, e a comparação entre políticas fica trivial. Proponho incluir f₄ em L antes do congelamento, ou então fixar a restrição K4 mais forte.

**Limite honesto.** f₃ e f₄ não dependem do clima, e f₂ só depende pelo estabelecimento. Logo todo o regret vem de f₁ e do pedaço de f₂. Se as janelas ZARC deixarem pouca variação, o resultado provável é "as políticas escolhem o mesmo plano", que é um resultado válido (protocolo, item 3).

## 3. Arrependimento e minimax regret

**Oráculo por cenário.** m_s = min_{x′∈X} L(x′,s). Isso exige avaliar todo X em cada cenário.

**Regret.** R(x,s) = L(x,s) − m_s ≥ 0, e R = 0 no plano oráculo.

**Minimax regret.** x_MMR = argmin_{x∈X} max_{s∈S_constr} R(x,s). Equivale ao programa mín t sujeito a t ≥ L(x,s) − m_s para todo s.
- **Desempate (emenda sugerida ao protocolo):** o protocolo usa só o ID lexicográfico. Proponho, em ordem: (i) menor regret máximo, com tolerância 10⁻¹²; (ii) menor regret médio; (iii) ID. Com (ii), x_MMR é **não dominado** no vetor de perdas (L(x,s))_s: se y domina x, então y tem regret máximo ≤ e regret médio estritamente menor, e x não é escolhido [T].
- **Invariâncias esperadas [T]:** somar uma constante a L em um cenário só muda m_s por essa constante e não muda R; multiplicar L por α > 0 multiplica R por α e não muda x_MMR; reordenar os cenários não muda nada.

**Políticas em condições equivalentes.** Todas usam o mesmo X, a mesma L, a mesma normalização (da construção), o mesmo snapshot e o mesmo avaliador; só a regra de escolha muda:
- **P0 (plano atual).** x₀ dado pelo usuário, ou o padrão regional da demo (soja + milho 2ª, nos 2 anos, [H]). Deve pertencer a X; o app valida K0–K4 sobre x₀. Se x₀ ∉ X, o app avisa a violação e **não** usa x₀ como política comparável; ela é mostrada só como referência fora de X, porque com oráculo em X o regret poderia ser negativo.
- **P1 (convencional).** x_CONV = argmin_x L(x, s̄), com s̄ = série diária média da construção por data de calendário (mês-dia), usada nos dois anos. Dia 29/fev entra em s̄ como a média só dos anos bissextos da construção e é ignorado em anos sem 29/fev. O código de indicadores é o mesmo usado nos cenários reais.
- **P1b (baseline forte).** x_EXP = argmin_x média_s L(x,s).
- **P2.** x_MMR.

No teste, todas são avaliadas pelas mesmas funções em S_teste, com o oráculo do próprio cenário de teste (só para medir). O avaliador recalcula L(x,s) a partir do plano, sem reaproveitar valores do otimizador.

**Dados ausentes.** Se qualquer −999 cair dentro de uma janela de uma opção, o indicador é ausente e o cenário inteiro sai do conjunto de todas as políticas (regra única, registrada em log).

## 4. Não dominância (Pareto) e alternativas distintas

**Vetor do Pareto.** F(x) = (f̄₁, f₁^wc, f̄₂, f̄₃, f̄₄), em que f̄ é a média sobre S_constr e f₁^wc = max_s f₁(x,s) é a pior exposição. Com isso a fronteira mostra o custo do compromisso entre desempenho médio e pior caso.

**Dominância.** x ≻ y se F_k(x) ≤ F_k(y) para todo k e F_k(x) < F_k(y) para algum k. Comparo com F arredondado a 9 casas, para não criar dominância por ruído de ponto flutuante. Planos com F idêntico formam uma classe de equivalência (um representante, com a lista dos demais).

**Cálculo (exato).** Deduplicar vetores F, ordenar lexicograficamente e varrer mantendo a lista da fronteira (complexidade O(n·|fronteira|)). A fronteira esperada é pequena (dezenas ou centenas).

**Relação com as políticas.**
- x_EXP e todo minimizador de soma ponderada com w > 0 estão na fronteira (no espaço de f̄; nos cinco eixos podem ser só fracamente eficientes).
- x_MMR é não dominado no espaço (L(·,s))_s (item 3), mas **pode ficar fora** da fronteira de F. Se ficar, o app diz isso: "dominado em F; escolhido por minimizar o regret máximo".
- x_CONV não tem garantia nenhuma. Se estiver dominado, isso aparece.

**Alternativas distintas (garantia explícita).**
1. A assinatura do plano é σ(x) = (c₁,c₂,c₃,c₄), só com a cultura. Datas e grupos de ciclo são refinamentos e não contam como alternativa nova.
2. Da fronteira, mantém-se um representante por assinatura: o de menor Σ w₀ F, com desempate por ID.
3. Exibem-se K = 4–5 alternativas. Entram primeiro P0, x_CONV, x_EXP e x_MMR (um só se coincidirem, avisando) e depois os extremos (argmin de cada f̄_k). O resto vem por seleção do ponto mais distante (max–min) em distância de Hamming entre assinaturas; empate pela distância euclidiana em F normalizado.
4. Exigência: distância de Hamming mínima δ = 2 janelas entre quaisquer duas alternativas exibidas, relaxada a 1 se não houver candidatos, com aviso. Se existirem menos assinaturas que K, mostra-se menos; **nunca** se completa com duplicatas.

**"Quando a recomendação muda" (barato, sai da mesma enumeração).** Varrer w sobre a malha do simplex (passo 0,1: 286 pontos) e registrar x_EXP(w), x_MMR(w) e as fronteiras em que o plano muda; e a recomendação sob ρ_max = 20% vs 30%. Não usa o teste.

## 5. Método: enumeração exaustiva vs solver

| Critério | Enumeração exaustiva (numpy) | MILP / CP-SAT |
|---|---|---|
| Exatidão | Ótimo exato, e fronteira de Pareto exata | Ótimo exato, mas a fronteira exige múltiplas resoluções |
| Oráculo m_s por cenário | Sai direto (mín por coluna) | Exige uma resolução por cenário (42) |
| Minimax regret | argmin do máx por plano, trivial | Variável t, restrições por cenário, linearização de C2 (maior intervalo sem cobertura) e dos limiares, com risco de erro |
| Verificação independente | Avaliador ingênuo em laços Python | Mesmo, mais o modelo |
| Dependências / tempo de implementação | Só numpy; poucas horas | Solver + modelo + testes de equivalência |
| Escala | Até ~5·10⁷ pares plano×cenário (estimativa) | Necessário só se X passar de ~10⁷ |

**Decisão:** enumeração exaustiva vetorizada; **nenhum solver no MVP**.
- **Por quê.** (i) O protocolo já exige X completo enumerado para o oráculo e para o teste. (ii) 4 janelas e catálogo pequeno dão |X| ≤ ~4·10⁵. (iii) Dá ótimo e Pareto exatos, sem folga de otimalidade para reportar (gap = 0 por construção). (iv) Correção é o ponto crítico e um laço de referência é fácil de escrever. (v) Solver só traria risco de linearização.
- **Estrutura.** Montar uma tabela A[o, ano] de indicadores por (opção, ano climático) — o f₁ de um plano é a soma de quatro entradas dessa tabela. O pedaço não aditivo (C1/C2, cobertura) é calculado no plano, em blocos de planos.
- **Quando mudaria a decisão.** Se o catálogo ou as datas de semeadura fizerem |X| passar de ~10⁷, usar CP-SAT sobre a tabela A (aí sim) e comparar com a enumeração numa instância reduzida.

**Bibliotecas Python mínimas.**
- Núcleo da otimização: `numpy`. Biblioteca padrão: `itertools`, `dataclasses`, `datetime`, `hashlib`, `json`, `tomllib`. `pandas` para as tabelas do snapshot e dos resultados.
- Teste: `pytest`.
- De outros papéis: `streamlit` e um gráfico (Software Engineer); `scipy.stats` (Wilcoxon) e `requests` (POWER), se o Evaluation e o MLOps usarem.
- Fora do MVP: `ortools`, `pulp`, `cvxpy`.

## Riscos de correção a testar em 14/11

Os T1–T7 do protocolo continuam valendo. Estes são adicionais ou específicos da formulação:

1. **Avaliador duplo.** Avaliador vetorizado vs avaliador ingênuo em laços sobre 200 planos aleatórios, incluindo cobertura com lacunas (C2) e anos bissextos: resultados idênticos até 10⁻⁹ [T].
2. **Viabilidade independente do clima.** X idêntico em todos os cenários; contagem de |X| e de descartes por restrição no log. Soja semeada em dezembro tem de inviabilizar milho/sorgo em J2 (R3 do Methods) [T].
3. **Propriedades do MMR.** Invariâncias do item 3 e não dominância de x_MMR; instância de brinquedo calculada à mão (T3 do protocolo) [T].
4. **Oráculo e monotonia.** R ≥ 0, R = 0 no oráculo, e m_s(ρ_max = 30%) ≤ m_s(20%), porque X cresce [T].
5. **Normalização só da construção.** Perturbar os anos de teste não muda mín/máx, x_CONV, x_EXP, x_MMR (T2 do protocolo) [T].
6. **Dados ausentes e 29/fev.** −999 vira ausente e remove o cenário igualmente para todas as políticas; s̄ tem o tratamento de 29/fev descrito no item 3 [T].
7. **Pareto.** Nenhum elemento da fronteira é dominado; nenhum elemento fora dela é não dominado; igualdade com ruído de ponto flutuante tratada. Teste com instância pequena por força bruta O(n²) [T].
8. **Alternativas distintas.** Hamming mínimo δ cumprido; nenhuma duplicata; menos que K assinaturas devolve menos que K [T].
9. **Degeneração.** Se f₁ for quase constante, as políticas coincidem e a fronteira colapsa. Reportar como resultado, não ajustar pesos para "criar" diferença [H].
10. **Artefato de ocupação e cobertura.** Verificar se o plano escolhido muda só com w₄ (sensibilidade W0–W4). Se mudar, o resultado depende do peso e deve ser dito assim [T].
11. **P0 ∈ X.** Padrão da demo e plano do usuário validados por K0–K4; fora de X, o app avisa e não compara como política [T].
12. **Desempenho.** Medir tempo e memória da enumeração completa com |X|·|S| real, antes de congelar o catálogo; acionar a guarda de tamanho se passar do limite [T].

## Pendências para decisão (até as 12h de 14/11)

- **Evaluation:** aceitar f₄ em L, o desempate (regret máximo → médio → ID) e a grade W0–W4.
- **Methods / Dev Lead:** ρ_max (20%), grupos de ciclo no MVP, margem m, matriz B e número de datas de semeadura por opção.
- **Não verificado por mim:** valores do catálogo, limiares dos indicadores e tamanhos reais de X.
