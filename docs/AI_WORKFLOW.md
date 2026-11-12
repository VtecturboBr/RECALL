# Workflow de criação e desenvolvimento — RECALL

Configuração consolidada em 12/11/2026, após três rodadas do arquivo INICIAR_SKILLS_E_WORKFLOW_ALEF.md (versão 1.1).

## Configuração confirmada

| Campo | Valor |
| --- | --- |
| Projeto | RECALL |
| Autonomia | A IA resolve detalhes e pergunta apenas o essencial |
| Prioridade | Máximo acabamento e qualidade |
| Exploração visual | Comparar três direções diferentes antes de mudanças importantes |
| Limites | Ferramentas locais gratuitas permitidas |
| Serviços pagos | Podem ser sugeridos, jamais contratados automaticamente |
| Refinamento | Até três ciclos por entrega |
| Formato | Resultado pronto e resumo curto |
| Continuidade | Arquivos versionados no GitHub VtecturboBr/RECALL |
| Skills | Todos os grupos habilitados para seleção automática conforme a tarefa |

## Como executar uma tarefa

1. **Recuperar contexto.** Consultar AGENTS.md, docs/AI_CHECKPOINT.md, escopo pedido e estado real da branch/código. Não supor que o checkpoint substitui o repositório atual.
2. **Selecionar capacidades.** Usar docs/AI_SKILLS.md e escolher skills específicas. Carregar instruções apenas quando necessárias. Confirmar requisitos e se elas estão mesmo instaladas, carregadas ou pendentes.
3. **Definir resultado.** Estabelecer três a cinco critérios observáveis: fluxo curto, correção, legibilidade, acessibilidade, persistência e desempenho relevantes.
4. **Comparar três direções quando houver decisão visual significativa.** Distinguir alternativas de verdade; aguardar escolha apenas quando a diferença afeta o rumo do produto. Alterações pequenas seguem a identidade aprovada sem exigir novas escolhas.
5. **Construir.** Implementar uma versão concreta e preservar recursos anteriores. Não acrescentar dashboards, páginas ou opções sem necessidade comprovada.
6. **Avaliar e refinar.** Até três ciclos (criar → inspecionar/testar → priorizar até três problemas → corrigir → comparar). Problemas que impeçam o requisito principal não são considerados mero refinamento opcional.
7. **Verificar.** Rodar os testes aplicáveis. Indicar separadamente revisão de código, testes automatizados e testes manuais realmente feitos.
8. **Entregar.** Resultado pronto, links, resumo curto, riscos e pendências verdadeiros; atualizar docs/AI_CHECKPOINT.md se o estado mudou.

## Regras de UX e produto

- Priorizar o caminho mais curto do registro à ação. A interface principal deve continuar compreensível em poucos segundos.
- Reduzir funcionalidades sobrepostas, sem apagar conteúdo nem remover capacidades essenciais silenciosamente.
- RECALL usa identidade própria; não importar grunge, horror anime ou estilos de outros projetos sem pedido.
- Evitar dependências pagas. Recursos locais gratuitos podem ser propostos; instalação no computador do usuário exige ação/consentimento apropriado.
- Trabalhar com commits/PRs rastreáveis; não declarar uma release como pronta só porque o build passou.
- Não publicar bancos de dados pessoais, credenciais, anotações privadas ou arquivos locais do usuário no Git.

## Módulos próprios do arquivo de inicialização

- **Direção visual:** recuperar a identidade do projeto, extrair paleta/typografia/movimento e validar renderização.
- **Continuidade LIN0:** quando relevante fora do RECALL, distinguir fatos confirmados, propostas, lacunas e contradições.
- **Criar → avaliar → refinar:** critérios verificáveis, até três ciclos, evidência do resultado.
- **Simplificação de produto:** unir funções redundantes, reduzir escolhas e preservar acessibilidade e dados.
- **Protótipo jogável LIN0:** um objetivo testável por vez, controles e feedback claros, validação real quando houver Studio.

Estes são módulos textuais fornecidos pelo usuário, não skills externas instaladas.

## Comandos de conversa

Os comandos são convenções do workflow, não comandos nativos obrigatórios da plataforma:

| Comando | Uso |
| --- | --- |
| /iniciar | Ler configuração e checkpoint, preparar capacidades |
| /skills | Mostrar estado verificado das skills e pendências |
| /configurar | Alterar preferências solicitadas |
| /workflow rapido | Menos planejamento; verificação essencial |
| /workflow equilibrado | Etapas curtas e até dois ciclos de refinamento |
| /workflow caprichado | Até três ciclos de refinamento e qualidade máxima |
| /referencias | Extrair regras de referências sem inventar observações |
| /direcoes | Comparar três direções distintas |
| /planejar | Objetivo, etapas, dependências e critérios |
| /criar | Executar tarefa concreta autorizada |
| /criticar | Inspecionar resultado e priorizar problemas |
| /refinar | Corrigir e comparar até o limite configurado |
| /simplificar | Identificar e reduzir sobreposições |
| /debug | Reproduzir, investigar e confirmar correção |
| /testar | Executar verificações reais e identificar limites |
| /canon | Separar fatos, propostas, lacunas e conflitos de LIN0 |
| /prototipo | Implementar uma pequena hipótese jogável |
| /status | O que está pronto, bloqueado e próximo |
| /checkpoint | Atualizar o arquivo de continuidade |
| /exportar-contexto | Resumo portátil para outra IA |
| /continuar | Retomar o último estado verificado |
| /ativar NOME | Habilitar entrada para seleção e verificar requisitos |
| /desativar NOME | Deixar de selecionar entrada; não desinstalar |

## Persistência e limites

A configuração vive nos arquivos deste repositório. Ela não cria memória permanente, sincronização automática entre chats/ChatGPT/Codex, instalações externas ou permissão para executar ações não solicitadas. Uma nova sessão deve ler estes arquivos na branch relevante. Em caso de conflito, prevalece a solicitação mais recente do usuário e as instruções mais prioritárias do ambiente.
