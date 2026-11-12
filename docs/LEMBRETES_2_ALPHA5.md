# RECALL v0.5 Alpha 5 — Lembretes 2.0

**Status:** implementação experimental na branch \`feature/v0.5-alpha5-lembretes2\`, baseada na Alpha 4.1, PR #6. Não promover a estável sem testes Windows reais.

## Funcionalidade

- **Notas e tarefas:** no Arquivo, selecione o item, informe "Me lembrar em", escolha a repetição e clique em "Salvar lembrete". Prazo e "Quando fazer?" de uma tarefa NÃO disparam aviso automaticamente; é preciso ativar o lembrete explicitamente.
- **Recorrência:** Uma vez, Todos os dias, Toda semana e Todo mês. Após marcar "Resolvido", uma ocorrência periódica avança diretamente para a próxima futura, pulando as vencidas durante o período offline. A hora local é preservada onde possível. Fins de mês são ajustados para o último dia válido e o ciclo seguinte retoma o dia original (31 jan → 28 fev → 31 mar).
- **Adiar:** um aviso vencido pode ser adiado por 15 min ou 1 h na área Agora. O backend também oferece 5, 30, 180 min e 1 dia, com validação. Adiar afeta apenas a ocorrência atual, não o padrão de repetição.
- **Histórico:** a seção "Lembretes" da tela Agora exibe a atenção pendente, os próximos e os últimos eventos (agendado, disparado, falha, vencido, adiado, resolvido e desativado). Nenhuma tela principal foi adicionada.
- **Offline-first:** toda a programação, fila e histórico ficam no SQLite; sem serviço remoto, mensalidade ou sincronização externa.
- **Recuperação:** ao iniciar RECALL ou manter o aplicativo na bandeja, o serviço Rust verifica vencimentos a cada ~15 segundos. Uma ocorrência muda atomicamente para "attention" antes do envio do aviso nativo. Mesmo se o Windows ocultar/bloquear o toast ou o aplicativo fechar nessa janela, o item continua na caixa "precisa de atenção".
- **Prevenção de duplicatas:** uma ocorrência só muda para "attention" uma vez; reiniciar não repete automaticamente o toast. Um novo toast só aparece após adiamento explícito ou novo ciclo recorrente. Uma fila grande/antiga é resumida para reduzir bombardeio de notificações.
- **Modelo de entrega:** o timestamp "dispatched_at" registra aceitação da API do sistema, NÃO comprova leitura nem entrega visual no Windows. A pessoa confirma manualmente em "Resolvido".

## Migração e segurança

O banco da Alpha 4.1 usa \`notes\`, \`tasks\`, \`task_plans\`, \`inbox_items\` e \`note_revisions\`. A Alpha 5 **não remove nem reescreve essas tabelas**, adicionando \`reminder_rules\` e \`reminder_events\`, índices e \`PRAGMA user_version=2\`.

Antes da primeira migração, é feito backup SQLite verificado \`recall-pre-reminders-*.sqlite\`. Bancos mais antigos sem \`note_revisions\` continuam usando seu backup pré-integridade existente antes da migração. Se o backup falhar, o aplicativo recusa a migração.

Lembretes anteriores das notas são importados via \`INSERT OR IGNORE\`:
- \`remind_at\` sem \`notified_at\` → "scheduled", inclusive se vencido; entra na caixa após iniciar.
- \`remind_at\` com \`notified_at\` → "attention" sem nova notificação automática (para não duplicar toasts históricos).
- Campos originais \`remind_at/notified_at\` permanecem disponíveis e são mantidos em sincronia nas ações novas, para não perder compatibilidade.
- Prazos \`tasks.due_at\` da Alpha 4 permanecem informativos e não viram notificações não solicitadas.
- Tarefas concluídas ou excluídas não são notificadas. Ao concluir tarefa, seu lembrete é cancelado sem excluir o histórico.

**Importante:** as notas e lembretes locais continuam na mesma pasta de dados e com o mesmo ID Tauri.

## Limites reais

**Não há notificações enquanto o EXE está totalmente encerrado ou o computador está desligado.** Não há serviço residente do Windows nem agendamento no Windows Task Scheduler. A recuperação é feita **ao abrir novamente o RECALL** e exige que o processo seja iniciado. Ocultar janela na bandeja mantém o processo funcionando.

Notificações nativas dependem de permissão/assistência de foco do Windows. O agendador marca "attention" antes do toast e preserva o item até a ação do usuário. Ele não promete entrega exatamente uma vez na central do sistema, porque crashes/permissões do SO estão fora do controle do aplicativo; garante persistência e ausência de retoast automático por ocorrência.

Os eventos são armazenados no banco e a interface mostra uma amostra recente. Não foi implementado clique na notificação para abrir a tarefa específica, nem sincronização na nuvem, nem calendário novo.

## Testes automatizados

\`\`\`powershell
npm run check
npm run test:unit
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
python -m unittest discover -s scripts -p "test_*.py"
\`\`\`

Cobertura Rust adicional: migração de notas antigas, proteção de repetição no mesmo vencimento, adiar e reativar, recorrência diária/mensal e meses curtos, persistência após reabrir SQLite, tarefas concluídas e compatibilidade das colunas anteriores.

## Testes manuais obrigatórios no Windows

1. Faça backup externo dos dados da Alpha 4.1; instale Alpha 5; confirme notas, tarefas, ideias, prazos e revisões intactos.
2. Na nota, agende aviso para 1 minuto no futuro; confirme popup do Windows e card persistente em Agora. Clique "Resolvido".
3. Agende aviso em tarefa, mantenha RECALL minimizado na bandeja; verifique aviso e permanência do registro.
4. Adie um aviso em 15 min; feche/reabra a janela (sem sair da bandeja) e verifique horário e ausência de duplicação.
5. Crie aviso diário/semanal/mensal; resolva uma ocorrência; confirme próximo horário correto e registro histórico.
6. Feche totalmente pelo menu da bandeja, aguarde o vencimento e reabra; verifique recuperação sem enxurrada de popups.
7. Bloqueie permissão de notificação no Windows; verifique que o aviso vencido aparece mesmo sem toast.
8. Conclua/exclua uma tarefa agendada; confirme que não dispara.
9. Verifique a restauração offline em um banco de **teste**, jamais diretamente nos dados reais.
10. Teste teclado, zoom, legibilidade da seção de lembretes e atalhos globais.

**Nenhum teste manual Windows deve ser marcado como aprovado sem realmente executá-lo.**
