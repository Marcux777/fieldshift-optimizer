# FieldShift — protocolo de avaliação retrospectiva (v1.1, congelada em 13/11/2026)

**Autor:** Evaluation & Benchmark Engineer.

**Situação:** escrito antes de qualquer resultado. Não há código nem execução.

**Fontes que mandam:**
- Decisões aprovadas pelo usuário em `/home/marcusvinicus/p/fieldshift-optimizer/docs/DECISIONS.md`. Elas valem sobre a v1.0.
- Termos e definições da formulação na seção "Revisão 13/11 noite" de `prep/optimization-engineer.md`, itens R1–R5. Este protocolo usa os mesmos nomes. Se houver divergência, ela precisa ser resolvida antes da 1ª execução.

**Regra de mudança:** qualquer alteração vira v1.2, com motivo, horário e novo SHA-256 registrados no canal **antes** da 1ª execução no teste. Mudança feita depois de ver o teste invalida a comparação, e isso tem de ser relatado.

**Marcas:** [V] verificado; [N] não verificado ou hipótese.

## Mudanças em relação à v1.0 (SHA-256 049037e2…7496)
1. A perda passa a ter três objetivos: g₁ exposição climática, g₂ cobertura vegetal e g₃ diversidade, com peso padrão w\* = (0,50; 0,25; 0,25). A grade de sensibilidade agora tem 15 pesos fixos; antes eram 3 a 5 vetores a definir. A normalização passou a ser teórica, em [0,1]. O único parâmetro calculado com dados são as medianas de e₃, que vêm só da construção.
2. O desempate do MMR, do CONV e do EXP agora é lexicográfico, com três critérios; antes era só pelo ID. Os dados ausentes seguem uma regra única para todos os cenários. Entram também o descarte de 29/fev e um portão de "dados insuficientes".
3. O ZARC ≤ 20% virou restrição fixa, sem relaxar, e a sensibilidade a 30% foi retirada. Se não houver nenhum plano viável, o resultado é `INVIAVEL`. Se o P0 for inviável, ele é relatado e sai da comparação de regret.
4. A radiação não é usada. Os anos de 1981 a 1983 continuam na construção. A ressalva "[N] POWER desde 1981" foi resolvida.
5. A célula principal é (-18,0/-50,625). A vizinha (-17,5/-51,25) serve só como sensibilidade opcional.
6. Resultados negativos e inconclusivos ficam com o mesmo destaque, em formato fixo (item 5).
7. Os testes T2, T3, T5, T6 e T7 foram atualizados. Não entrou nenhuma métrica nova.

## 1. Split temporal e regra de não vazamento
- **Cenário:** um par de anos agrícolas consecutivos (y, y+1). Cada ano agrícola vai de 1/set a 31/ago, e a avaliação usa o clima diário observado do NASA POWER na célula principal. J1–J2 usam o clima de y, e J3–J4 o de y+1.
- **Construção:** anos agrícolas de 1981/82 a 2009/10, o que dá 28 pares.
- **Teste congelado:** de 2010/11 a 2024/25, com 14 pares sobrepostos e 7 pares não sobrepostos.
- **Fronteira:** nenhum par cruza a passagem de 2009/10 para 2010/11. O ano 2025/26 fica fora.
- **Dados:** só `PRECTOTCORR`, `T2M`, `T2M_MAX` e `T2M_MIN`, sem radiação.
  - [V] O DECISIONS registra 16 436 dias sem −999 em `PRECTOTCORR` e `T2M` na célula principal, de 1981-01-01 a 2025-12-31.
  - [N] A completude de `T2M_MAX` e `T2M_MIN` nessa célula foi reportada pelo MLOps, mas tem de ser conferida no manifesto do snapshot.
- **O que só pode usar a construção:**
  - as medianas de e₃;
  - o limiar de ano seco (P25);
  - o clima médio s̄;
  - os limiares e frações de fase do Methods;
  - a escolha de todas as soluções, para w\* e para cada ponto da grade;
  - qualquer ajuste de catálogo ou de restrição.
- **Uso do teste:** os dados de teste entram uma vez, e só para medir. Se for preciso reexecutar, todas as execuções são relatadas.
- **Cálculo do MMR (só construção):**
  - X é o conjunto viável completo e enumerado.
  - L_w(x,s) = w₁g₁ + w₂g₂ + w₃g₃.
  - R(x,s) = L_w(x,s) − min_{x′∈X} L_w(x′,s).
  - x_MMR = argmin_{x∈X} max_{s∈S_constr} R(x,s).
- **Desempate determinístico** (o mesmo da Optimization, R5), com comparação em q(v) = round(v, 9) e soma em ordem fixa:
  - **x_MMR:** menor regret máximo, depois menor regret médio, depois menor ID do plano.
  - **x_CONV:** menor L(x, s̄), depois menor média de L, depois menor ID.
  - **x_EXP:** menor média de L, depois menor regret máximo, depois menor ID.
  - O ID é a tupla de índices do catálogo, em ordem lexicográfica, e não depende da ordem de enumeração.
  - A tolerância de 10⁻⁹ é só numérica e não se confunde com o ε estatístico do item 3.
- **Dados ausentes** (regra única para todos os cenários, independente do plano e da política):
  1. −999 e NaN viram ausente, nunca 0 mm.
  2. O dia 29/fev é descartado em todas as séries, então cada ano tem 365 dias.
  3. Um cenário só vale se as quatro variáveis estiverem completas em todo o par. Um cenário inválido sai para todas as políticas e para o oráculo, e o motivo vai para o log.
  4. s̄ é a média por (mês, dia) dos anos válidos da construção, sem imputação. Se algum dia de s̄ ficar sem valor, a execução aborta.
  5. Se restarem menos de 20 dos 28 pares de construção, ou menos de 10 dos 14 pares de teste, o resultado é "dados insuficientes" e não há veredito [N: os limiares são convenção].
- **Vazamento residual declarado:**
  - As janelas do ZARC foram calibradas com anos que se sobrepõem ao teste. Por isso entram como restrição fixa, igual para todos.
  - A correção do `PRECTOTCORR` pode usar uma climatologia longa [N].

## 2. Políticas sob as mesmas condições
| Política | Escolha (só com a construção) |
|---|---|
| P0 Plano atual | É informado pelo usuário. Na demo: soja GI no dec29 + milho 2ª GI, nos 2 anos (Methods) [N]. Fica registrado na configuração antes de rodar e nunca é ajustado. |
| P1 Convencional | x_CONV: minimiza L em s̄, aplicado aos dois anos. |
| P2 Minimax regret | x_MMR, conforme o item 1. |
| P1b Baseline forte | x_EXP: minimiza a média de L sobre S_constr. |

- **Paridade (gate):** todas as políticas usam o mesmo snapshot e a mesma célula, o mesmo X e as mesmas restrições. Isso inclui a classe de solo, o ZARC ≤ 20% e a regra de 2ª safra opcional sem sobreposição, essa última aplicada como o verificador da Optimization definir. Também são os mesmos w, L, medianas, cenários válidos e avaliador. O avaliador é separado do otimizador e recalcula L a partir do plano.
- **Inviabilidade:**
  - Se X = ∅, o resultado é `INVIAVEL`, com diagnóstico, e a avaliação não roda. Isso é um resultado registrado, não uma falha. O limite de 20%, o grupo e o solo nunca são relaxados.
  - Se P0 ∉ X, o relatório diz "plano atual inviável sob ZARC ≤ 20%". O P0 sai das comparações de regret, e P1, P1b e P2 seguem.
- **Força da conclusão:** superar o P1 sozinho é evidência fraca, porque s̄ suaviza os veranicos. A frase "MMR mais robusto" exige comparar também com o P1b.

## 3. Métricas e limiares (cenários de teste válidos, peso w\*)
O regret no teste usa como oráculo o min_{x′∈X} L do próprio cenário de teste, só para medir.

| ID | Definição | Leitura |
|---|---|---|
| M0 Viabilidade | Fração dos planos de P1, P1b e P2 aprovados pelo verificador | Gate: precisa ser 100% |
| M1 Regret máximo | max_{s∈teste} R(x,s) | Métrica primária |
| M2 Regret típico | Média e mediana de R(x,s) no teste | Custo típico |
| M3 Anos secos | Média de L e de R nos pares cujo ano 1 ou 2 tem chuva de out a mar abaixo do P25 da **construção** | Informar n; se n < 3, só descritivo |
| M4 Estabilidade | (a) Fração de pares com R ≤ ε. (b) IQR de L. (c) Hamming médio entre o plano completo e os planos de leave-one-pair-out na construção | (c) não usa o teste |
| M5 Preço da robustez | L(x_MMR, s̄) − L(x_CONV, s̄) na construção | Custo da robustez no clima médio |

- **Comparação pareada (P2 contra P1b e P2 contra P1):**
  - D_s = R(P2,s) − R(Pk,s). Há empate quando |D_s| ≤ ε, com ε = 0,02 na escala [0,1] [N: convenção].
  - Relatar vitórias, empates e derrotas, a mediana de D e o IC 95% por bootstrap em blocos de 2 pares, com 10 000 réplicas e seed 20261114.
  - Wilcoxon nos 7 pares não sobrepostos. O menor p bilateral possível é 0,0156, então o poder é baixo.
- **Classificação:**
  - **Sustentado:** o IC 95% de D fica todo abaixo de 0, o M1 de P2 é menor que o de Pk em pelo menos 10% relativo e pelo menos ε, e M0 = 100%.
  - **Negativo:** o IC fica todo acima de 0, ou o M1 de P2 é maior ou igual ao de Pk.
  - **Inconclusivo:** todos os outros casos, inclusive quando as políticas escolhem planos idênticos. Isso é esperado se g₁ variar pouco, e é resultado válido.
- **Sensibilidade** (só relatada; nunca serve para escolher nem muda o veredito principal, que usa w\*):
  - **Grade de 15 pesos.** Confirmo a interpretação: são as triplas (a,b,c)/4 com a, b, c inteiros ≥ 0 e a+b+c = 4, o que dá C(6,2) = 15. A lista e a ordem são as do R2 da Optimization, e a lista inclui w\*. Em cada ponto, as políticas são recalculadas na construção e medidas no teste. A tabela sai inteira, com M1, M2 e a classificação. Doze pontos têm algum peso zero, e neles os empates são esperados e resolvidos pelo desempate.
  - **Célula vizinha (-17,5/-51,25), opcional:** mesmo protocolo, mesmo split, mesmas regras. Se o veredito mudar, isso tem de ser relatado.
  - **Solo AD3–AD6, opcional:** mede só |X| e as recomendações.

## 4. Checagens de vazamento e reprodutibilidade (14–15/11)
1. O snapshot da célula principal tem SHA-256 no manifesto, com data, URL, parâmetros, versão da API e coordenada. O hash é verificado no início, e a execução é offline.
2. A configuração congelada tem SHA-256 publicado no canal antes da 1ª execução no teste. Ela inclui split, w\*, a grade de 15, ε, P0, seed, solo, limiares de dados insuficientes e regra de 2ª safra.
3. A construção recebe dados já filtrados. Uma asserção confere que a maior data é anterior a 1/set/2010 e que nenhum par cruza a fronteira.
4. As medianas de e₃, o P25 seco e s̄ vêm só da construção e são persistidos com hash. Nenhum mínimo ou máximo é tirado dos dados, e uma asserção confere que 0 ≤ g_k ≤ 1 em todo (x,s).
5. O log registra a contagem de ausentes por variável e ano, os cenários excluídos e o motivo, igual para todas as políticas.
6. O log registra |X|, os planos descartados por restrição com o motivo, e o resultado `INVIAVEL` com diagnóstico quando ocorrer.
7. Cada execução registra commit, lockfile, versão do Python, seed (usada só no bootstrap), horário e o hash do JSON de resultados.
8. Uma segunda execução limpa tem de dar o mesmo hash.
9. Todas as execuções no teste vão para `reports/eval_runs.md`, inclusive as que falharam e as reexecuções.

## 5. Resultado negativo, inconclusivo ou inviável, e afirmações proibidas
- **Mesmo destaque e mesmo formato para qualquer desfecho.** A primeira linha do relatório traz a classificação (Sustentado, Negativo, Inconclusivo, `INVIAVEL` ou "dados insuficientes"). Depois vêm a tabela M0–M5 das políticas, com n e IC, e uma frase literal. Exemplo: "No teste 2010/11–2024/25, célula (-18,0/-50,625), o minimax regret não reduziu o arrependimento máximo frente à otimização por perda esperada (IC 95% de D: [a, b])."
  - Um resultado negativo não vai para nota de rodapé nem fica abaixo de um positivo da grade.
  - A grade e a célula vizinha nunca substituem o veredito em w\*.
- **Vão para Limitações:** desvios do protocolo, reexecuções, P0 inviável, vazamentos residuais e a variação entre células.
- **Afirmações proibidas sem nova evidência:**
  - produtividade, lucro, receita, economia de água ou de N, recuperação do solo, carbono;
  - que o MMR é melhor ou mais robusto em geral, fora de Rio Verde, de sequeiro, da célula e do período testados, ou com pesos fora da grade;
  - validade para o clima futuro;
  - "significativo" sem o teste e o IC;
  - conformidade com o ZARC vigente para fins de seguro ou crédito;
  - ineditismo;
  - qualquer número que não tenha vindo de uma execução registrada.
- **Auditoria antes de publicar:** nenhum ganho vai para o Scientific Writer sem minha auditoria dos artefatos brutos.

## Testes críticos para 14/11 (harness de avaliação)
- **T1 Split:** nenhum dado de teste chega ao otimizador, e nenhum par cruza a fronteira.
- **T2 Invariância ao teste:** perturbar ou embaralhar os anos de teste não altera x_CONV, x_EXP, x_MMR, as medianas de e₃, o P25 nem s̄, em w\* e em todos os 15 pesos.
- **T3 Instância de brinquedo calculada à mão** (3 planos × 3 cenários, com empates exatos):
  - MMR, CONV e EXP batem com o cálculo à mão, inclusive nos três níveis de desempate;
  - embaralhar a ordem de enumeração e a de cenários não muda nenhuma escolha.
- **T4 Regret:** R ≥ 0 sempre, e R = 0 no oráculo de cada cenário.
- **T5 Viabilidade:**
  - o verificador aprova P1, P1b e P2;
  - rejeita um plano com decêndio a 30% e um plano com sobreposição entre J1 e J2;
  - um P0 inviável é relatado e excluído das comparações;
  - um solo sem decêndio a 20% devolve `INVIAVEL`, sem relaxar o limite.
- **T6 Ausentes:**
  - um −999 injetado exclui aquele cenário para todas as políticas e para o oráculo, e não vira 0 mm;
  - o dia 29/fev é descartado;
  - um dia de s̄ sem valor faz a execução abortar;
  - abaixo do portão de dados, o resultado é "dados insuficientes".
- **T7 Determinismo e configuração:**
  - duas execuções dão o mesmo hash de resultados;
  - um snapshot com hash errado aborta a execução;
  - a grade tem 15 pontos únicos, todos somando 1, e inclui w\*.
