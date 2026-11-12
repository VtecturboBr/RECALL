# Catálogo de skills e roteamento por tarefa

Estado auditado em 12/11/2026. Fonte: arquivo INICIAR_SKILLS_E_WORKFLOW_ALEF.md (v1.1).

**Todos os grupos estão HABILITADOS para seleção por tarefa.** Isso é uma preferência de roteamento: não significa que arquivos, scripts ou dependências tenham sido instalados.

## Skills externas referenciadas

| Entrada | Origem | Selecionar para | Estado verificado |
| --- | --- | --- | --- |
| Impeccable | https://github.com/pbakaus/impeccable | critique, distill, polish, animate, adapt e audit de interface | Pendente: não instalada nem carregada |
| UI UX Pro Max | https://github.com/nextlevelbuilder/ui-ux-pro-max-skill | Direção visual e sistema de design | Pendente: não instalada nem carregada |
| MengTo — design/interatividade | https://github.com/MengTo/Skills | Motion, WebGL, GSAP, design-first, 3D | Pendente: não instalada nem carregada |
| MengTo — jogos web | https://github.com/MengTo/Skills | Câmera, combate, level design, desempenho web | Pendente: não instalada nem carregada |
| Superpowers — systematic-debugging | https://github.com/obra/superpowers | Reproduzir falhas e encontrar causa raiz | Pendente: não instalada nem carregada |
| Superpowers — verification-before-completion | https://github.com/obra/superpowers | Exigir evidência antes de declarar pronto | Pendente: não instalada nem carregada |
| Webapp Testing | https://github.com/anthropics/skills/tree/main/skills/webapp-testing | Playwright, fluxos e screenshots testáveis | Pendente: não instalada nem carregada |
| React Best Practices | https://github.com/vercel-labs/agent-skills | React/Next.js quando houver React; RECALL atual usa Svelte | Pendente: não instalada nem carregada |
| Remotion Skills | https://github.com/remotion-dev/skills | Animações/vídeos em código quando o projeto pedir | Pendente: não instalada nem carregada |

Não presumir caminho, versão, commit nem disponibilidade de scripts. Verificar README, arquivos completos e dependências atuais na fonte oficial antes de executar. Não copiar coleções inteiras indiscriminadamente. Respeitar licenças se algum arquivo for incorporado.

Na sessão de configuração havia um plugin Product Design disponível e a instrução de roteamento dele foi lida; isso é diferente de instalar as skills acima e não garante disponibilidade em futuras sessões.

## Módulos próprios disponíveis

Os cinco módulos do documento do usuário são orientações textuais: direção-visual-alef, continuidade-lin0, criar-avaliar-refinar, simplificacao-de-produto e prototipo-jogavel-lin0. Aplicá-los por tarefa conforme docs/AI_WORKFLOW.md. Estado: módulo próprio; não exige instalação externa.

## Matriz de seleção

| Situação | Selecionar |
| --- | --- |
| Design ou redesign do RECALL | Direção visual + simplificação de produto; Impeccable/UI UX Pro Max somente após carregar instruções reais |
| Bug | systematic-debugging, se carregada; senão procedimento equivalente com reprodução e logs |
| Entrega de código | verification-before-completion, se carregada; exigir testes mesmo sem ela |
| Teste visual de interface | Webapp Testing, se ferramenta e navegador estiverem disponíveis |
| React/Next | React Best Practices, apenas se o projeto realmente usar React |
| Jogos web/Roblox | MengTo para web; para Roblox, ferramentas Luau/Studio e instruções específicas |
| Vídeos em código | Remotion Skills somente quando vídeo for objetivo |
| Qualquer resultado criativo | Módulo criar-avaliar-refinar com até três ciclos |

## Revalidar status

Use uma destas condições verificáveis:
- **Instalada:** o arquivo/caminho persistente e requisitos existem e foram verificados.
- **Carregada na sessão:** instruções completas lidas nesta sessão; verificar de novo no próximo ambiente.
- **Módulo próprio:** disponível textualmente no workflow.
- **Pendente:** fonte conhecida, mas sem instalação/leitura/requisitos suficientes.
- **Desativada:** escolha explícita do usuário.

Registre separadamente se uma skill foi de fato **executada/testada**. Nunca marque instalada somente por ter encontrado o repositório GitHub.
