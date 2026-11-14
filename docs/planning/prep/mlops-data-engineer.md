# Preparação MLOps & Data Engineer — dados NASA e snapshot (13/11/2026)

Somente planejamento e leituras de API. Nada foi gravado no repositório nem em `data/`; os testes ficaram em `/tmp/pw/` (apagado ao final).
[V] = verificado hoje (com fonte ou teste de leitura); [N] = não verificado ou inferência; [S] = sugestão minha.

## 1. NASA POWER: variáveis, resolução, período, ausências

**Endpoint testado** [V]: `https://power.larc.nasa.gov/api/temporal/daily/point?parameters=...&community=AG&longitude=...&latitude=...&start=YYYYMMDD&end=YYYYMMDD&format=JSON`. Resposta: API "POWER Daily API" **v2.10.3**, título "Source Native Resolution Daily Data", `fill_value -999.0`, `time_standard LST` (padrão). Documentação: https://power.larc.nasa.gov/docs/services/api/temporal/daily/ (máx. 20 parâmetros por requisição pontual; dados diários de 1981-01-01 até quase tempo real). Dicionário de parâmetros (lido via API, 152 parâmetros diários): https://power.larc.nasa.gov/api/system/manager/parameters?community=ag&temporal=daily

| Parâmetro | Unidade [V] | Papel | Ausências no teste 1981–2025 [V] |
|---|---|---|---|
| `PRECTOTCORR` | mm/dia | chuva (MERRA-2 corrigida por viés) | 0 |
| `T2M`, `T2M_MAX`, `T2M_MIN` | °C | temperatura média/máx./mín. a 2 m | 0 |
| `RH2M` | % | umidade relativa | 0 |
| `QV2M`, `T2MDEW` | g/kg, °C | umidade específica, ponto de orvalho | 0 |
| `WS2M` | m/s | vento a 2 m | 0 |
| `EVLAND` | mm/dia | evaporação sobre terra | 0 |
| `EVPTRNS` | MJ/m²/dia | fluxo de energia de evapotranspiração | 0 |
| `GWETROOT`, `GWETTOP`, `GWETPROF` | 1 (0–1) | umidade do solo: zona radicular (~1 m), 5 cm, perfil | 0 |
| `ALLSKY_SFC_SW_DWN` | MJ/m²/dia | radiação de onda curta incidente | **1095 (1981–1983)** |
| `PS` | kPa | pressão | 0 |

- [V] **Não existe parâmetro `PET`/ET0**: o pedido com `PET` retorna HTTP 422 ("One of your parameters is incorrect: PET"). `EVLAND` é evaporação do modelo, não ET0 de referência. Se o Methods Scientist quiser ET0, será **calculada** (ver abaixo), não baixada.
- [V] **Radiação só existe a partir de 1984-01-01.** Para datas anteriores a API devolve -999 (os 1095 dias = 1981-01-01 a 1983-12-31) e, em pedido só de datas antigas, emite a mensagem "radiation parameters are not available until 1984-1-1". As outras variáveis cobrem 1981-01-01 a 2025-12-31 completas: 16 436 dias, 0 ausentes [V]. **Impacto:** o protocolo de avaliação começa em 1981/82; qualquer indicador que dependa de radiação (ET0 de Penman-Monteith) deixa os cenários 1981/82–1983/84 incompletos. Opções: [S] (a) basear E1–E6 em chuva, T2M_MAX/MIN, RH2M, EVLAND, GWETROOT (sem lacunas) e, se precisar de ET0, usar Hargreaves-Samani (só Tmax/Tmin + radiação extraterrestre calculada de latitude/data); ou (b) exigir radiação e excluir igualmente os cenários afetados para todas as políticas (o Optimization Engineer já propõe essa regra). Decisão do Methods/Evaluation antes das 12h de 14/11.
- [V] **Resolução espacial real**: os 3 pontos de Rio Verde nas coordenadas -17.80/-50.93, -17.90/-50.90 e -17.7981/-50.9292 e também -17.80/-50.55 devolvem **séries de chuva idênticas** (0 dias diferentes em 16 436), ou seja, o ponto "encaixa" numa célula de grade. Os limites foram achados por teste: a célula vai de lat -17.75 a -18.25 e lon aproximadamente -50.94 a -50.31 (`lon -50.95` e `lat -17.70` já caem em outras células). Isso é coerente com a grade nativa do MERRA-2 (0,5° lat × 0,625° lon, centro em -18.0/-50.625) [N: o número 0,625° é inferência do teste e do conhecimento do MERRA-2; a página da API cita "0.5° × 0.5°"]. A radiação vem de outras fontes (a resposta lista `MERRA2, SRB, SYN1DEG, POWER`) com resolução própria [N: não conferi].
- [V] **Resolução temporal**: diária; padrão `LST` (hora solar local). Manter o padrão e registrar no manifesto; `time-standard=UTC` é opção, mas muda o corte do dia.
- [V] **Latência**: pedido até 12/11/2026 retornou dados até 10/11; 11 e 12/11 vêm com -999 (tempo quase real ainda não preenchido). Por isso o snapshot deve terminar em data fixa (proposta: 2025-12-31), nunca "até hoje".
- **Tratamento de -999** [S]: o conversor trata -999 como `NaN`, nunca como 0 mm; a verificação conta ausentes por variável e data e **falha** se houver ausência fora das esperadas (radiação 1981–1983 listada no manifesto como lacuna conhecida). Cenário com ausência é excluído igualmente de todas as políticas (alinhado à nota do Optimization Engineer). Sem interpolação silenciosa.
- **Limite de uso** [V]: a documentação avisa que, com a grade de 0,5°, basta **uma requisição por célula**; repetir pedidos para o mesmo local pode causar bloqueio. Portanto: coleta única, nunca em laço nem em teste.
- Teste de tempo [V]: uma requisição de 45 anos × 10 parâmetros levou ~3 s e ~2,7 MB de JSON.

## 2. Outros dados NASA: corte para o MVP

| Fonte | Login? | Peso | Veredito |
|---|---|---|---|
| **NASA POWER** | Não [V] | ~2,7 MB para 45 anos × 10 variáveis [V] | **Dentro do MVP** |
| **GPM/IMERG** (chuva 0,1°) | Sim: o arquivo diário V07 redireciona (HTTP 302) para `urs.earthdata.nasa.gov` [V] | um arquivo global por dia; 25 anos = milhares de arquivos, exige recorte por subconjunto [N] | **Fora do MVP.** Opcional pós-MVP só se o usuário fornecer credencial Earthdata |
| **SMAP** (umidade do solo, NSIDC) | Sim: 302 para Earthdata [V] | HDF5 por dia; série só desde 2015 [N] | **Fora do MVP**: série curta demais para 28 pares de construção |
| **MODIS** (NDVI/ET; LP DAAC) | Sim: 302 para Earthdata [V] | HDF por tile e por 8/16 dias [N] | **Fora do MVP**: exige tile, mosaico e máscara de qualidade; sem ganho para o objetivo do MVP |

- [V] A busca de **metadados** no CMR (`https://cmr.earthdata.nasa.gov/search/granules.json`) é pública; o **download** dos arquivos não é. Não há `~/.netrc` nem variável Earthdata no ambiente [V].
- [S] Corte: **só NASA POWER**. Para o critério "pelo menos um dado NASA" basta o POWER (confirmado no plano de preparação). A limitação (uma célula, reanálise, sem validação com IMERG/SMAP) entra nas limitações do app.
- Fontes: https://www.earthdata.nasa.gov/ · https://cmr.earthdata.nasa.gov/search/ · https://urs.earthdata.nasa.gov/

## 3. Desenho do snapshot versionado [S] (nada criado)

**Estrutura** (a criar em 14/11 a partir das 9h, em `data/snapshots/`):
```
data/snapshots/<snapshot_id>/
  raw/power_daily_<cell>.json.gz   # resposta bruta, intacta (fonte da verdade)
  daily.parquet                    # derivado: 1 linha/dia, colunas = parâmetros, -999 -> NaN
  manifest.json
  SHA256SUMS
data/snapshots/current.txt         # aponta o snapshot_id vigente
```
- **Formato**: JSON bruto compactado (proveniência) + Parquet (leitura). ~16 436 linhas × 10 colunas, bem menor que 1 MB; cabe no repositório e no Streamlit Community Cloud. [S] CSV opcional só para auditoria humana.
- **`snapshot_id`** = `power-daily-<versao_api>-<celula>-<sha256 dos bytes brutos [:12]>`. O mesmo conteúdo gera o mesmo id.
- **`manifest.json`** (campos):
  - `snapshot_id`, `created_utc`, `collected_utc` (data da coleta; também é a "data de acesso" exigida na citação)
  - `source`: serviço ("NASA POWER Daily API"), `api_version` (lida de `header.api.version`, hoje 2.10.3), `sources` (lista de `header.sources`), `community`, `time_standard`
  - `request`: URL completa, `parameters`, latitude/longitude pedidas, latitude/longitude/elevação devolvidas em `geometry` (a célula), `start`, `end`
  - `fill_value` (-999) e `missing_counts` por variável (+ lacunas conhecidas, ex.: radiação 1981–1983)
  - `units` por variável (lidas de `parameters`), `rows`, `date_min`, `date_max`
  - `sha256` do bruto e do Parquet; `schema_hash`; `tool_versions` (Python, pandas, pyarrow, imagem Docker)
  - `citation` e `acknowledgement` prontos (ver abaixo); `license_note`
- **Execução offline**: o app, o otimizador, o avaliador e o `pytest` **nunca** acessam a rede. Existe um único passo de coleta (`fetch_snapshot`), executado só por humano/CI explícito. Em container: coleta com rede; testes e app com `docker run --network none`, o que torna a regra verificável (o teste falha se algo tentar a rede). Se o snapshot faltar ou o hash divergir do manifesto, o app para com erro claro, sem clima sintético nem fallback.
- **Idempotência**: a coleta grava em diretório temporário, valida (dias completos, unidades, ausências esperadas, hash) e só então renomeia para `<snapshot_id>`; se já existir snapshot com o mesmo pedido e o mesmo hash, não faz nada; se o hash do mesmo pedido mudar (NASA revisou dados), cria **novo** snapshot, nunca sobrescreve, e `current.txt` só muda por decisão explícita.
- **Validações na fronteira** (T1 do Software Engineer): 16 436 datas contíguas sem duplicata; só os parâmetros e unidades esperados; faixas físicas (chuva ≥ 0, Tmin ≤ T2M ≤ Tmax, umidade 0–100, GWET 0–1); -999 → NaN; hash confere.
- **Citação e licença** [V] (https://power.larc.nasa.gov/docs/referencing/): em qualquer trabalho ou publicação usar a *POWER's Reference*: "The data was obtained from National Aeronautics and Space Administration (NASA) Langley Research Center's Prediction Of Worldwide Energy Resources (POWER) project funded through the NASA Earth Science Division." E a *Data Reference* com **nome do serviço, número da versão e data de acesso** ("The data was obtained from the POWER Project's Daily 2.10.3 version on YYYY/MM/DD", ajustando "Hourly" para "Daily" [S]). A página também *pede* (não exige) avisar o projeto por e-mail (larc-power-project@mail.nasa.gov) sobre usos publicados. **Licença formal**: não encontrei texto de licença na página consultada [N]; não afirmar licença específica. Colocar a citação no app, no README e no campo "NASA Data" da submissão. O texto da citação virá do manifesto, para não divergir.

## 4. Coordenadas de Rio Verde

- [V] **Município**: Rio Verde, GO, **código IBGE 5218805** (API IBGE `https://servicodados.ibge.gov.br/api/v1/localidades/municipios/5218805`); área 8 374 km²; caixa envolvente lat -18.35 a -17.09, lon -51.73 a -50.36; centroide da malha IBGE (-17.7392, -51.0385) (https://servicodados.ibge.gov.br/api/v3/malhas/municipios/5218805/metadados). O código bate com o do recorte ZARC do Methods Scientist.
- [V] **Sede**: Wikidata Q942064, coordenada (-17.79778, -50.92778) via SPARQL (https://www.wikidata.org/wiki/Q942064, `P625`); OpenStreetMap/Nominatim dá o centro em (-17.7921, -50.9191). Concordam entre si em ~0,01°.
- [V — achado importante] **O município cobre várias células POWER**, e a sede cai a **0,0085° da borda oeste** de uma célula: -50.93 e -50.9292 caem na célula centrada em (-18.0, -50.625), mas -50.95 cairia na vizinha. O centroide IBGE (-17.74, -51.04) cai em **outra** célula (centro (-17.5, -51.25), elevação 815,8 m contra 625,3 m).
- [V] **Diferença entre as duas células** (chuva diária 1981–2025): correlação mensal 0,96; correlação do total set–ago por ano agrícola 0,82; média set–ago 1350 mm (célula da sede) contra 1428 mm; diferença máxima anual de 674 mm; 12 116 dias com valores diferentes. A escolha da célula **muda os números**.
- [S] **Decisão proposta**: pedir à API as coordenadas do **centro da célula** (lat -18.0, lon -50.625), não o ponto da sede, para não depender da fronteira (verificado: o pedido no centro devolve série idêntica à do ponto da sede, 0 dias de diferença). Registrar no manifesto o ponto da sede como referência e a célula como a unidade real da análise. Declarar na limitação que a série representa uma célula de ~55 × 66 km, não a propriedade. Se o usuário informar uma localização de propriedade, o app usa só a célula que a contém (e o snapshot cobre apenas as células coletadas; sem coleta online no app). Opcional: coletar a célula vizinha (centro (-17.5, -51.25)) como análise de sensibilidade da escolha da célula; o custo é uma requisição.
- **Como confirmar** [S]: (1) conferir a coordenada da sede em duas fontes independentes (IBGE/Wikidata/OSM, já feito: concordam); (2) conferir a `geometry` devolvida pela API e a elevação no manifesto; (3) teste automatizado: o pedido no centro e o pedido na sede devolvem a mesma série (impede deriva de célula); (4) o Methods Scientist informa se a classe de solo/ZARC se refere ao município inteiro (a tábua ZARC é municipal) para decidir se a célula da sede é representativa.

## Bloqueios e pendências

1. **Tempo**: nada de projeto, `fetch_snapshot` ou arquivo em `data/` antes de 14/11 às 9h; a coleta real (1 requisição) leva segundos.
2. **Decisões antes das 12h de 14/11** (Methods + Evaluation + Optimization): (a) lista final de variáveis (com ou sem radiação/ET0); (b) tratamento dos cenários 1981–1983 se radiação for exigida; (c) célula única (sede) ou célula + vizinha como sensibilidade.
3. **Credenciais**: nenhuma para o POWER. IMERG, SMAP e MODIS exigem conta Earthdata do usuário; ficam fora do MVP salvo decisão contrária.
4. **Não verificado**: licença formal do POWER; resolução espacial da radiação (SRB/SYN1DEG); o valor 0,625° como grade MERRA-2 (inferido do teste); tamanho e disponibilidade de IMERG/SMAP/MODIS para o recorte; versão da API pode mudar até 14/11 (por isso a versão é lida da resposta, nunca fixada no código).
5. **Container**: a coleta e os testes devem rodar no container do Software Engineer; aplicar `--network none` ao app e aos testes (ver `prep/container-plan.md`).
6. **Arquivos temporários**: `/tmp/pw/` (respostas de teste do POWER), apagado ao final desta tarefa. Nenhuma alteração no repositório do projeto.
