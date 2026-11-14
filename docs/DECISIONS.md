# Decisões do cliente (escopo técnico congelado em 13/11/2026)

Aprovadas pelo usuário. Mudança só com nova aprovação dele. Sem novas funcionalidades.

1. **ZARC.** Risco máximo de 20% como restrição principal; o limite não sobe automaticamente. Um grupo de ciclo validado por cultura, com as tabelas oficiais do ZARC para Rio Verde e a classe de solo documentada. Sem solução viável, reportar a inviabilidade.
2. **Clima.** Sem radiação solar no objetivo do MVP. NASA POWER: precipitação (`PRECTOTCORR`) e temperatura. 1981–1983 preservados para a análise meteorológica, desde que completos. Não preencher radiação inexistente.
3. **Geografia.** Uma célula NASA POWER como representação regional principal, inicialmente (-18,0 / -50,625). Uma célula vizinha é só sensibilidade opcional.
4. **Otimização.** Segunda safra opcional quando inviável, sem sobreposição temporal. Minimax regret com desempate determinístico. Dados ausentes tratados de forma explícita. Normalizações fixas e grade de 15 combinações de pesos, padrão 0,50 / 0,25 / 0,25 para exposição climática, cobertura vegetal e diversidade.
5. **Infraestrutura.** Python e Streamlit com deploy direto no Streamlit Community Cloud. Docker só se agregar valor a testes e reprodução. Sem serviço pago.
6. **Repositório.** `Marcux777/fieldshift-optimizer`, público, licença MIT para o código. Criação e implementação específicas só após a abertura autorizada (14/11 às 9h).
7. **Validação.** Plano atual, otimização por condições médias e minimax regret em cenários equivalentes. Dados de seleção e de teste separados. Resultados negativos e inconclusivos registrados.
8. **Conformidade.** Confirmar que a fonte NASA consta nos recursos elegíveis do desafio (NASA POWER consta). Registrar atribuições e a declaração de uso de IA.

Pendências externas (usuário): confirmação da equipe oficial, identidade e fuso do criador, criação do repositório, hospedagem e envio final.

Prioridade: 1) elegibilidade da fonte e regras agronômicas; 2) dados e motor; 3) validação; 4) interface, documentação e submissão. Se uma dependência comprometer a entrega, reduzir o escopo e reportar o que deixou de ser validado.

## Verificação feita em 13/11/2026

- NASA POWER, ponto (-18,0 / -50,625): HTTP 200, `PRECTOTCORR` e `T2M`, 16 436 dias de 1981-01-01 a 2025-12-31, nenhum -999. Leitura em memória, nada gravado.

## Interpretações do Development Lead (a confirmar pelo usuário)

- **Grade de 15 pesos:** todas as combinações de três pesos em múltiplos de 0,25 que somam 1 (incluindo 0,50/0,25/0,25). Dá exatamente 15 pontos.
- **Três objetivos:** exposição climática, cobertura vegetal e diversidade. A ocupação das janelas de 2ª safra, que a Optimization propôs como quarto objetivo, sai dos pesos; a 2ª safra opcional é tratada como restrição/opção.
- **Container:** usado em desenvolvimento e testes, para reprodução, conforme o pedido de 13/11 ("use um container"). O deploy não usa Docker (Streamlit Community Cloud instala a partir do repositório, então o repositório precisa trazer a lista de dependências e o snapshot).
