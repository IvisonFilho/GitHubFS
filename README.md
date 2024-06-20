# GitHubFS
GitHubFS: Sistema de Arquivos em Modo Usuário com Crate FUSE para GitHub

# About

FUSE-Rust é uma biblioteca crate Rust para implementação fácil de sistemas de arquivos FUSE em modo usuário.

FUSE-Rust não apenas fornece bindings, é uma reescrita da biblioteca FUSE original em C para aproveitar completamente a arquitetura do Rust.

Esta biblioteca foi originalmente derivada da crate fuse com a intenção de continuar o desenvolvimento, especialmente adicionando funcionalidades de ABIs após a versão 7.19.

# Detalhes sobre FUSE e FUSE-Rust

Um sistema de arquivos FUSE consiste em três partes essenciais:

1. Driver do Kernel: Este componente registra um sistema de arquivos no kernel e encaminha operações de arquivos para um processo de espaço de usuário que as manipula.

2. Biblioteca de Espaço de Usuário (libfuse): Essa biblioteca auxilia o processo de espaço de usuário a estabelecer e manter comunicação com o driver do kernel, facilitando a implementação de operações específicas do sistema de arquivos.

3. Implementação de Espaço de Usuário: Aqui é onde as operações do sistema de arquivos são efetivamente processadas pelo desenvolvedor, definindo como o sistema de arquivos FUSE irá se comportar na prática.

O projeto FUSE fornece o driver do kernel, enquanto a biblioteca libfuse é comumente usada para facilitar a comunicação entre o driver do kernel e o processo de espaço de usuário. No entanto, FUSE-Rust oferece uma abordagem diferente ao fornecer uma substituição para libfuse, toda escrita em Rust.

FUSE-Rust:

Substituição de libfuse: FUSE-Rust se posiciona entre o driver do kernel e a implementação de espaço de usuário, permitindo que desenvolvedores construam sistemas de arquivos FUSE diretamente em Rust. Isso possibilita aproveitar completamente a interface de tipos e os recursos de tempo de execução oferecidos por Rust.

Menos dependência de libfuse: Exceto por duas funções específicas - uma para configurar (montar) o sistema de arquivos FUSE e outra para desmontá-lo - que ainda dependem de libfuse, todas as operações principais são executadas em Rust. Em ambientes Linux, essas chamadas para libfuse podem ser opcionalmente removidas compilando sem o recurso "libfuse".

Essa abordagem com FUSE-Rust não apenas aproveita as capacidades avançadas de Rust, mas também oferece uma maneira mais integrada e autônoma de desenvolver sistemas de arquivos FUSE, mantendo a flexibilidade de usar partes da libfuse quando necessário.
