# Instalador RECALL para Windows

## Baixar v0.5 alpha 1 (Windows x64)

1. Abra https://github.com/VtecturboBr/RECALL/actions/workflows/windows-installer.yml .
2. Entre em **Gerar instalador RECALL Windows** e abra uma execução bem-sucedida da branch feature/v0.5.
3. Na seção **Artifacts**, baixe o arquivo ZIP **RECALL-v0.5-alpha1-Windows-x64**.
4. Extraia o ZIP e execute o arquivo com final **-setup.exe**.
5. Se desejar, confira a integridade com o SHA-256 salvo em **SHA256SUMS.txt**.

O workflow é executado automaticamente a cada push em feature/v0.5 e pode ser iniciado manualmente com **Run workflow**. O arquivo aparece na seção Artifacts da execução, não na aba Releases. O artefato é retido por 14 dias.

## Segurança e compatibilidade

- Build compilado no Windows x64 com Tauri 2 e instalador NSIS.
- Instalação para o usuário atual sem solicitar privilégios de administrador, salvo restrições do sistema.
- O aplicativo continua usando o identificador interno app.memoriaexterna.desktop, preservando o local do SQLite da v0.4.
- O instalador é **não assinado**: o SmartScreen pode avisar sobre origem desconhecida. Confira sempre a procedência no GitHub.
- Esta é uma **alpha**. Faça backup de notas importantes antes de instalar, e valide captura, lembretes, bandeja e janelas flutuantes.
- TESTAR_AGORA.html é uma demonstração da v0.4, não da v0.5.
- Nenhuma nota pessoal nem banco SQLite é enviado para o GitHub Actions.

## Compilação local (opcional)

Depois de instalar Node.js, Rust e pré-requisitos Tauri no Windows, rode npm install e npm run installer -- --bundles nsis. O executável de instalação estará em src-tauri/target/release/bundle/nsis/.

O GitHub Actions usa ações de cache e verificações do frontend e Rust antes de publicar o instalador. Se falhar, consulte a etapa vermelha da execução do workflow.
