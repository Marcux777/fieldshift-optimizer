# Fontes e parâmetros de dados

Atualizado em 14/11/2026 (Methods Scientist). Marcas: **[V]** verificado na fonte citada; **[N]** não verificado ou hipótese.
O manifesto do snapshot é a fonte de verdade para hashes e datas do dado NASA; esta página só resume.

## NASA (única fonte climática do MVP)

| Item | Valor |
| --- | --- |
| Fonte | NASA POWER, Daily API, point, community AG [V manifesto] |
| Ponto pedido / devolvido | (-18,0 / -50,625), elevação 625,31 m; célula que representa Rio Verde (decisão 3) [V manifesto] |
| Variáveis | `PRECTOTCORR` (mm/dia), `T2M`, `T2M_MAX`, `T2M_MIN` (°C) [V manifesto]; `T2M` não entra nos indicadores |
| Período | 1981-01-01 a 2025-12-31, 16 436 dias, 0 valores -999 [V manifesto] |
| Versão / coleta | API v2.10.3, fontes MERRA2 + POWER, coletado em 2026-11-14T17:58:06Z [V manifesto] |
| Snapshot | `data/snapshots/power-daily-2.10.3-lat-18_lon-50.625-46c276253b44/manifest.json` (hashes do bruto e do Parquet lá dentro) |
| Citação | Texto exigido pelo POWER em `manifest.json` → `citation`; guia: https://power.larc.nasa.gov/docs/referencing/ |
| Não usado | Radiação (`ALLSKY_SFC_SW_DWN` só existe desde 1984; decisão 2), IMERG/SMAP/MODIS (exigem login Earthdata) |
| Resolução | Grade MERRA-2 ~0,5° × 0,625° [N: inferida dos testes, não lida na documentação] |

Elegibilidade: o NASA POWER consta nos recursos do desafio (decisão 8; verificado pelo Development Lead em 13/11).

## Agronômicas

`…` = https://www.gov.br/agricultura/pt-br/assuntos/riscos-seguro/programa-nacional-de-zoneamento-agricola-de-risco-climatico

| Fonte | Uso | URL | Estado |
| --- | --- | --- | --- |
| Tábua de Risco ZARC 2026/27 (MAPA, dados abertos, CC-BY), recorte de Rio Verde | Decêndios de semeadura a 20% por cultura, grupo e solo | https://dados.agricultura.gov.br/dataset/tabua-de-risco-zoneamento-agricola-de-risco-climatico | [V] cópia em `config/zarc/`, SHA-256 `88f24986…18ad`; procedência em `config/zarc/SOURCE.md` |
| Dicionário de dados da Tábua de Risco | Códigos de ciclo, solo e manejo | `config/zarc/dicionario-de-dados-tabua-de-risco-2026.pdf` | [V] |
| Portaria SPA/MAPA 368/2025 (ZARC soja GO) | Ciclos 100/115/130 d, CAD AD1–AD6, Tmax > 40 °C, Tmin ≤ 3 °C, emergência 5–10 d | …/portarias/2025-2026/goias-go/PORTN368SOJAGO.ret2.pdf | [V] parâmetros; a tabela 2026/27 vem da Port. 198/2026, lida só via tábua |
| Portaria SPA/MAPA 178/2025 (ZARC milho 1ª GO) | Grupo I: ciclo 100 d, fases 15/35/30/20 d; geada Tmin ≤ 2 °C | …/portarias/2025-2026/goias-go/PORTN178MILHO1SAFRAGO.ret2.pdf | [V] |
| Portaria SPA/MAPA 386/2025 (ZARC milho 2ª GO) | Grupo I ≤ 110 d; geada ≤ 2 °C; ≥ 6 dias de chuva no decêndio final | …/portarias/2025-2026/goias-go/copy_of_PORTN33.PDF | [V] |
| Portaria SPA/MAPA 400/2025 (milho 2ª + braquiária GO) | Grupo I ≤ 115 d | …/portarias/2025-2026/goias-go/PO91CB1.PDF | [V] |
| Portaria SPA/MAPA 123/2025 (sorgo granífero GO) | Grupo I ≤ 110 d; Tmax > 38 °C; Tmin < 2 °C | …/portarias/2025-2026/goias-go/PORTN123SORGOGRANIFEROGO.pdf | [V] |
| IN Agrodefesa 6/2024 | Semeadura de soja em GO de 25/set a 02/jan; vazio sanitário de 27/jun a 24/set | https://goias.gov.br/agrodefesa/wp-content/uploads/sites/49/2014/11/Instrucao-Normativa-Soja_Agrodefesa-No-06-de-05-de-agosto-de-2024-1-1.pdf | [V] texto oficial lido em 14/11 |
| IN Agrodefesa 5/2026 | Início excepcional em 20/set só na safra 2026/27 | só notícias, ex. https://comprerural.com/agrodefesa-antecipa-inicio-da-semeadura-da-soja-em-goias-para-20-de-setembro/ | [N] texto oficial não encontrado; sem efeito no modelo (dec27 está a 40%) |
| Embrapa Algodão, Comunicado Técnico 377 (2016) | Sorgo mais tolerante ao déficit hídrico e mais precoce que o milho; braquiária como cobertura | https://www.infoteca.cnptia.embrapa.br/infoteca/bitstream/doc/1066067/1/Sistemasdecultivodeplantasdecobertura.pdf | [V] |
| Embrapa Soja, FBN | Soja dispensa N mineral com inoculação | https://www.infoteca.cnptia.embrapa.br/infoteca/bitstream/doc/447053/1/CNPSOCOM.TEC.7400.pdf | [N] não lido por inteiro; só contexto, fora do motor |

## Solo

| Fonte | Uso | URL | Estado |
| --- | --- | --- | --- |
| Usuário (parâmetro) | Classe AD1–AD6 e textura escolhem a linha do ZARC | `config/soil.toml` | padrão AD4 + argilosa = [N] hipótese |
| ISRIC SoilGrids 2.0 (REST) | Contexto: 29–51% de argila em dois pontos rurais de Rio Verde (consulta de 13/11) | https://rest.isric.org/soilgrids/v2.0/docs | [V] consulta; fora do motor |
| IN SPA/MAPA 1/2022 | Função de pedotransferência granulometria → AD | citada na Port. 368/2025 | [N] não lida, não implementada |

## Vazamento residual (só declarável)
- A tábua ZARC foi calibrada com ≥ 15 anos de estações que se sobrepõem aos anos de teste; é usada como calendário fixo, igual para todas as políticas.
- A correção de chuva do `PRECTOTCORR` pode ter usado o período inteiro [N].
