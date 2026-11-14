# RECALL Alpha 10.4 — Recuperação de rascunhos do Quadro

Estado: PR experimental separado, base Alpha 10.3 (PR #27). Não realizar merge na main.

## Princípio

Uma falha de salvamento ou conflito de edição em múltiplas janelas não deve apagar o texto do usuário.
SQLite com revisão CAS permanece a fonte autoritativa; snapshots no localStorage são uma camada auxiliar.

## Implementado

- Adição de identificação de editor e cópias concorrentes usando draftOwnership.ts já existente.
- Rascunhos contêm boardId, kind, conteúdo, posição, tamanho, revision e identificador de proprietário.
- Nunca restaurar automaticamente rascunho com revisão incompatível ou pertencente a outra janela.
- Exibir rascunhos antigos, órfãos, concorrentes, inválidos ou não salvos em painel de recuperação.
- Para notas, links e tabelas válidas: **Criar cartão com esta cópia** sem sobrescrever a versão do banco.
- Para imagem/vínculo: apresentar somente cópia visível para consulta (não clonar arquivos nem relações).
- Descartar cópia requer confirmação explícita e comparação exata do snapshot, sem tocar no SQLite.
- O código de salvamento usa o snapshot específico lido antes da chamada Rust para evitar limpar
  alterações mais recentes produzidas enquanto a operação ainda está em curso.
- `src/lib/boardRecovery.ts` encapsula parsing defensivo, classificação e comparação, sem acessar DOM.
- `tests/boardRecovery.test.mjs` cobre legados v10.0, revisão diferente, concorrência, órfãos e dados inválidos.

## Roteiro automatizado

1. npm ci, npm run check, npm run test:unit, build web; Python restore; cargo check/test; NSIS SHA256.
2. Com rascunho de mesma revisão, confirmar restauração local seguida de salvamento CAS.
3. Com revisão divergente, garantir que o SQLite não seja sobrescrito e o usuário veja uma cópia.
4. Simular duas janelas e provar que limpar o primeiro snapshot não remove o segundo.
5. Simular salvamento falho e excluir cartão: a exclusão deve continuar bloqueada.
6. Imagens e vínculos não devem gerar cartões incompletos ou quebrados na recuperação.
7. Executar smoke test manual posterior em WebView2/Windows, usando banco de teste.

## Restrições

- Esta fase **não inclui** desfazer/refazer; será implementado depois com histórico transacional por ação.
- Para imagens e vínculos, a recuperação manual ainda exibe o snapshot, sem restauração automática.
- O armazenamento de recuperação é local e tem limitações de quota e permissões do WebView.
- Não migrar banco: a versão SQLite permanece v10 e builds anteriores não podem abrir o banco atualizado.
- Não instalar em cima da única cópia dos dados e não realizar merge na main.
