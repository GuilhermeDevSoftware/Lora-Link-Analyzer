# LoRa Link Analyzer

Projeto de estudo e portfólio para desenvolver um analisador de enlace LoRa com **STM32 e Rust embarcado**, **ESP32 e C** e análise de dados no computador.

## Objetivo

Transmitir pacotes de teste entre dois rádios LoRa e avaliar a qualidade da comunicação. O ESP32 ficará conectado ao computador por USB, permitindo coletar os resultados e, futuramente, processá-los com Python.

## Arquitetura planejada

| Parte | Hardware | Software | Função |
| --- | --- | --- | --- |
| Nó remoto | STM32 Nucleo-F446RE + SX1278 | Rust embarcado | Enviar pacotes de teste e receber confirmações |
| Base | ESP32 + SX1278 | C com ESP-IDF | Receber pacotes, responder e encaminhar resultados ao computador |
| Computador | PC conectado à base por USB | Python, futuramente | Armazenar, analisar e visualizar resultados |

Os dois módulos disponíveis utilizam o chip SX1278 e operam em 433 MHz. A comunicação planejada é LoRa ponto a ponto.

## Estado atual

**A validação inicial dos microcontroladores foi concluída. Os módulos LoRa ainda não foram conectados nem testados neste projeto.**

### Concluído

- [x] Preparação do Rust no Windows e instalação do alvo `thumbv7em-none-eabihf`.
- [x] Configuração do probe-rs e do driver ST-LINK.
- [x] Compilação e gravação do firmware na Nucleo-F446RE.
- [x] Controle do LED LD2, conectado ao PA5, com temporização via SysTick.
- [x] Execução confirmada por duas piscadas rápidas seguidas de uma pausa.
- [x] Ajuste do alinhamento da seção de código em `memory.x`.
- [x] Compilação e gravação do exemplo `hello_world` no ESP32.
- [x] Recebimento das mensagens do ESP32 pelo monitor serial no WSL.

### Próximos passos

- [ ] Conferir a pinagem e conectar os módulos LoRa.
- [ ] Validar a comunicação entre cada microcontrolador e seu rádio.
- [ ] Configurar parâmetros de rádio compatíveis.
- [ ] Transmitir e receber os primeiros pacotes.
- [ ] Implementar confirmação de recebimento.
- [ ] Coletar RSSI, SNR e estatísticas de entrega de pacotes.
- [ ] Desenvolver a análise e os gráficos no computador.

## Estrutura atual

| Caminho | Conteúdo |
| --- | --- |
| `stm32-blink/stm32-blink/` | Firmware inicial da Nucleo-F446RE em Rust |
| `esp32-hello/` | Exemplo inicial do ESP32 em C |
| `README.md` | Documentação geral |

A pasta dupla `stm32-blink` corresponde à estrutura atual da extração do projeto.

## Firmware STM32

No terminal do Windows, a partir da raiz do repositório:

```powershell
cd stm32-blink/stm32-blink
rustup target add thumbv7em-none-eabihf
cargo build
cargo run
```

A Nucleo deve estar conectada pela porta USB do ST-LINK. O comando `cargo run` grava o firmware e inicia a execução na placa.

O firmware utiliza `stm32f4xx-hal 0.23.0`, clock interno HSI e atrasos com SysTick. O arquivo `Cargo.lock` registra as versões resolvidas das dependências.

| Arquivo | Finalidade |
| --- | --- |
| `Cargo.toml` | Dependências e perfis de compilação |
| `.cargo/config.toml` | Alvo ARM, linker e comando de gravação |
| `memory.x` | Mapa de memória e alinhamento da seção de código |
| `build.rs` | Disponibiliza o mapa de memória ao linker |
| `src/main.rs` | Configuração dos periféricos e controle do LED |

## Firmware ESP32

No WSL, com o ESP-IDF instalado, a partir da raiz do repositório:

```bash
source ~/esp/esp-idf/export.sh
cd esp32-hello
idf.py build
idf.py -p /dev/ttyUSB0 flash monitor
```

Na primeira configuração de uma cópia nova, execute `idf.py set-target esp32` antes de `idf.py build`. Adapte o caminho da instalação do ESP-IDF e a porta serial ao seu ambiente. O dispositivo USB precisa estar acessível no WSL.

O código inicial foi copiado do exemplo `examples/get-started/hello_world` do ESP-IDF. Ele imprime `Hello world!`, mostra informações do chip e reinicia após uma contagem. Esse reinício é parte do exemplo.

Para encerrar o monitor serial, pressione **Ctrl + ]**.

## Ambiente validado

| Ferramenta | Versão |
| --- | --- |
| Rust | 1.98.0 |
| Cargo | 1.98.0 |
| probe-rs | 0.32.0 |
| ESP-IDF | 6.0.1 |
| stm32f4xx-hal | 0.23.0 |

O firmware STM32 é desenvolvido no Windows; o firmware ESP32, no Ubuntu/WSL. As pastas `target/` e `build/` contêm arquivos gerados e não devem ser versionadas.

## Aprendizados envolvidos

Rust embarcado, C, GPIO, temporização, gravação e depuração, comunicação SPI, rádio LoRa e análise de dados.

## Autor

Guilherme Costa
