# RECALL Alpha 4 — Matriz de testes manuais no Windows

**Estado inicial:** todos os itens **não executados pela IA**. O usuário confirmou ter aberto a Alpha 4, mas não comunicou resultados específicos.

**Preparação:** use uma base de teste ou faça backup SQLite consistente antes de interferir em dados reais. Registre versão, sistema, hora local, permissões de notificação, estado da bandeja e resultado para cada cenário.

| ID | Cenário / ação | Resultado esperado | Estado |
| --- | --- | --- | --- |
| W01 | Abrir app com notas v0.4/v0.5 e reiniciar | Nenhum registro perdido ou alterado sem pedido | Não executado |
| W02 | Captura global Ctrl+Shift+Espaço -> Ideia -> Ctrl+Enter | Salva uma vez, aparece em Arquivo, mantém rascunho após erro | Não executado |
| W03 | Captura global -> Tarefa "Revisar amanhã às 15h" | Prévia correta, bucket Depois, data persistida | Não executado |
| W04 | Criar "Estudar semana que vem" | Prévia inequívoca sobre existência/ausência de data | Não executado; R09 |
| W05 | Criar "Revisar às 09h" quando agora são 17h | Não interpretar prazo passado sem aviso claro | Não executado; R10 |
| W06 | Abrir tarefa em Depois, definir data, avançar relógio para esse dia | Aparece em Hoje exatamente no dia esperado | Não executado |
| W07 | Clicar nos contadores Depois e Algum dia | Abre apenas tarefas do período clicado | Não executado; R11 |
| W08 | Editar título da nota em janela principal e corpo em cartão simultaneamente | Nenhum campo se perde; conflito sinalizado | Não executado; R02 |
| W09 | No cartão, inserir título >250 caracteres e clicar fechar | Impede fechamento ou oferece rascunho recuperável | Não executado; R01 |
| W10 | Cortar acesso ao banco durante salvamento | Texto preservado; erro não desaparece antes de resolver | Não executado |
| W11 | Criar lembrete para 1 minuto, negar permissão de notificação | Item continua reconhecível como pendente/não confirmado | Não executado; R03 |
| W12 | Criar 40 lembretes e deixar vencer durante app fechado | Sem tempestade de avisos; sem duplicados; pendências visíveis | Não executado; R03 |
| W13 | Fechar janela principal e recuperar pela bandeja | Processo continua e atalho funciona | Não executado |
| W14 | Apresentação com teclado, Tab, Shift+Tab, Esc | Foco contido/restaurado e contador acessível | Não executado; R13 |
| W15 | Excluir nota/tarefa/ideia por engano | Há desfazer ou recuperação previsível | Não executado; R15 |
| W16 | Instalar por cima da Alpha 3 e reabrir notas, cartões e tarefas | Identidade/local de banco preservados | Não executado |
| W17 | 1.000 notas longas + busca e edição | Medir latência e memória; definir orçamento antes de otimizar | Não executado; R12 |
| W18 | Zoom da WebView e navegação apenas por teclado | Texto legível, contraste e foco visível sem corte | Não executado; R13/R19 |

**Registre:** data, versão, passos, comportamento real, captura/log (se permitido), severidade, conclusão e resultado após correção. Se o cenário não puder ser executado, mantenha "não executado", não marque "passou".

### Regressões obrigatórias antes de qualquer merge

- Notas, tarefas e ideias intactas; nenhum esquema removido sem migração documentada.
- Captura inline e global; cartões flutuantes; eventos de atualização entre janelas.
- Lembretes locais e tray; permissões de notificação; reinício.
- Datas relativas e absolutas, mudança de dia, horário passado, foco e período.
- Instalação e desinstalação NSIS; preservação de AppData quando aplicável.
