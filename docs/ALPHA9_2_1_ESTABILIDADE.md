# RECALL Alpha 9.2.1 — Estabilidade e Integridade

**Versão:** `0.5.0-alpha.9.2.1` — Windows x64 experimental.  
**Branch:** `fix/alpha9-2-1-integridade-estabilidade`; **PR #21 (draft)**.  
**Base:** Alpha 9.2, PR #20. **Nunca fazer merge automático na `main`.**

## Correções de código

| Auditoria | Correção aplicada |
|---|---|
| F01 | Captura Rápida preserva digitação posterior ao início do salvamento, inclusive ao trocar o tipo |
| F02 | Heartbeat nativo Rust a cada ~12 s independente da aba; incremento persistido limitado a 20 s evita contagem de horas após suspensão |
| F03 | Sessão ativa de tarefa fora de Hoje mostra controles de pausa/finalização na tela Agora |
| F04 | Ao concluir/excluir tarefa, resumo armazenado no cache é enviado e persistido na mesma transação que encerra a sessão |
| F05 | Script offline recupera um original órfão único e validado; Rust impede criar banco vazio se uma restauração ficou interrompida |
| F06 | Links filtram apenas destinos ativos ANTES de 50 e são consultados de uma vez, sem N+1 |
| F07 | Excluir destaques de revisão ocorre antes de selecionar 12; painel busca próximos candidatos quando se oculta uma sugestão |
| F08 | Backup SQLite pode ser criado em conflito editorial; UI avisa explicitamente sobre rascunhos não persistidos |
| F09 | Rascunhos possuem tag de instância e uma janela não descarta diretamente cache pertencente à outra; substituições mantêm uma cópia de recuperação por dono anterior |
| F10 | Captura Rápida de tarefa vazia usa placeholder comum `Nova tarefa` |
| F11 | Converter ideia em nota/tarefa propaga relações ativas na mesma transação; registros anteriores ficam preservados |
| F12 | Upgrade de versão anterior à v5 faz um único snapshot completo obrigatório antes da cadeia de migrações, em vez de 3 cópias equivalentes; não apaga backups existentes |
| F13 | Consulta de links elimina N+1; revisão não transfere corpos completos de tarefas |
| F16 | Restaurador confere versão, colunas essenciais, tabelas por schema e integridade referencial |
| F17 | Cópia de tarefa em conflito aceita título vazio com `Nova tarefa` |
| F18 | README atualizado para versão funcional e processo real de build |

## Verificações exigidas antes de disponibilizar como estável

- **F14 (E2E):** a CI cobre regras Rust, Node e Python, mas não substitui cliques e múltiplas janelas no Windows. Não declarar E2E concluído sem executá-lo.
- **F15 (build):** persistir `package-lock.json` e `Cargo.lock`, instalar via `npm ci` / `cargo --locked`, e confirmar a geração com o mesmo conjunto de dependências. Não confundir checksum do instalador com build bit a bit reproduzível.
- **F09:** os caches de recuperação de donos anteriores ficam em `localStorage`, com prefixo `recall-draft-recovery:`, para evitar sobrescritas; uma futura interface de recuperação de rascunhos pode ser implementada sem mexer no banco.
- **F12:** ainda não há limpeza automática de histórico ou backups, intencionalmente. A redução evita redundância em novas migrações. Histórico grande pode consumir espaço; futura ferramenta de gerenciamento deve requerer consentimento e backup.
- **F13:** listas gerais ainda transferem registros completos. Antes de importar milhares de notas/tarefas, medir latência e considerar paginação, sem mudar semântica de pesquisa.
- **F18:** arquivos Svelte legados permanecem no repositório; remover somente após revisão visual e confirmação de ausência de imports externos.

## Regressões automatizadas

- Captura salva texto original e mantém texto digitado na espera da chamada ao SQLite.
- Rascunhos de um dono diferente não são removidos e têm cópia de recuperação se substituídos.
- Conversão de ideias em notas e tarefas conserva relações ativas; as históricas continuam registradas.
- Mais de 50 relacionamentos recentes excluídos não escondem relacionamentos ativos antigos.
- Ocultando 12 entre 20 destaques, os oito restantes passam a aparecer.
- Concluir/excluir tarefa com sessão ativa preserva o resumo em `focus_sessions`.
- Banco v4/v5/v6 migra até v7 preservando o conteúdo e com pontos de recuperação validados.
- Interrupção após primeira renomeação de restore é recuperável; múltiplos originais são recusados.
- Backup futuro/incompleto/estruturalmente incompatível é recusado antes de substituir o banco real.
- App não cria banco vazio se detectar `.recall-original-*.sqlite`.

## Teste integrado único no Windows

1. Fechar RECALL por **Sair do aplicativo** no ícone da bandeja.
2. Copiar **toda** `%APPDATA%\app.memoriaexterna.desktop`, preservando `-wal`, `-shm`, backups e arquivos ocultos. Preferir **outra conta do Windows** com cópia dos dados.
3. Baixar o artefato **RECALL-v0.5-alpha9-2-1-estabilidade-Windows-x64** do workflow Windows desta branch **somente depois de Success**. Verificar `SHA256SUMS.txt`.
4. Abrir a Captura Rápida, começar a salvar algo, digitar novo texto imediatamente; o texto novo não deve sumir. Criar tarefa sem título na captura e na interface principal.
5. Iniciar foco em uma tarefa, mudar para Arquivo e trabalhar por 3 minutos. Retornar: duração deve refletir a sessão ativa segundo a política de heartbeat.
6. Enquanto há foco ativo, mover tarefa para Depois/Algum dia; voltar ao Agora e pausar/finalizar pelo controle da sessão pendente.
7. Escrever resumo do foco, concluir/excluir a tarefa de teste sem pressionar Pausar: o resumo precisa aparecer no histórico.
8. Associar 60+ itens, excluir 50+ destinos; demais conexões devem continuar visíveis. Converter uma ideia conectada em nota/tarefa e conferir vínculos.
9. Criar 20 pendências simuladas e ocultar 12; conferir que as oito restantes são mostradas.
10. Criar um conflito de edição entre nota flutuante e editor principal; fazer backup, confirmando aviso de que o rascunho não persistido não entra no SQLite.
11. Simular restaurador **apenas em diretório de teste**: renomear o banco de teste para `.recall-original-simulado.sqlite`, confirmar bloqueio de banco vazio e recuperação pelo script offline.
12. Reiniciar, verificar lembretes, apresentações, revisão diária, links e integridade dos registros; observar erros de console e dados reais apenas em banco de teste.

## Segurança e limitações

O SQLite v7 continua sem novas migrações. **Nunca tente abrir banco v7 por versão antiga**. Restore deve ser feito com o aplicativo encerrado e arquivos WAL/SHM preservados. Windows pode bloquear notificações se permissões do sistema estiverem desativadas; nenhuma garantia de toast durante desligamento.

Não apagar automaticamente histórico, backups ou rascunhos, nem enviar arquivos pessoais ao GitHub. CI verde e instalador gerado **não** confirmam a experiência real WebView2. Este PR fica em rascunho até o teste manual.
