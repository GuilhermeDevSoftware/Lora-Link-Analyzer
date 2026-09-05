# LoRa Link Analyzer

Projeto de estudo e portfólio para desenvolver um analisador de enlace LoRa utilizando **STM32 com Rust embarcado**, **ESP32 com C/ESP-IDF** e, futuramente, ferramentas em Python para armazenamento, análise e visualização dos resultados.

## Objetivo

Transmitir pacotes de teste entre dois rádios LoRa e avaliar a qualidade da comunicação por meio de métricas como:

- RSSI;
- SNR;
- pacotes recebidos e perdidos;
- pacotes duplicados ou fora de ordem;
- taxa de entrega;
- latência, em uma etapa futura.

O ESP32 funciona como gateway conectado ao computador por USB. A STM32 atua como nó remoto e transmite os pacotes de teste.

## Arquitetura

| Parte | Hardware | Software | Função |
| --- | --- | --- | --- |
| Nó remoto | STM32 Nucleo-F446RE + SX1278 | Rust embarcado | Gerar e transmitir pacotes LoRa sequenciais |
| Base | ESP32 + SX1278 | C com ESP-IDF | Receber os pacotes e calcular métricas do enlace |
| Computador | PC conectado à base por USB | Monitor serial e Python, futuramente | Armazenar, analisar e visualizar os resultados |

Os dois módulos utilizam o SX1278 e operam em **433 MHz**. A comunicação atual é LoRa ponto a ponto.

## Estado atual

Em **05/09/2026**, foi concluída a primeira comunicação LoRa pelo ar entre a STM32 Nucleo-F446RE e o ESP32.

O enlace atualmente executa o seguinte fluxo:

1. A STM32 gera um pacote com número sequencial.
2. O SX1278 conectado à STM32 transmite o pacote em 433 MHz.
3. O SX1278 conectado ao ESP32 recebe o pacote.
4. O ESP32 extrai o número de sequência.
5. O gateway calcula métricas e apresenta os resultados no monitor serial.

Exemplo de pacote:

```text
SEQ=00012;MSG=STM32-LORA
```

Exemplo de saída no ESP32:

```text
Pacote recebido: SEQ=00012;MSG=STM32-LORA
Tamanho: 24 bytes
Pacotes recebidos: 9
Pacotes perdidos: 1
Pacotes duplicados: 0
Fora de ordem: 0
Taxa de entrega: 90.00%
RSSI: -105.0 dBm
SNR: 9.25 dB
```

Também foi realizado um teste controlado no qual a STM32 pulou propositalmente uma sequência. O ESP32 identificou corretamente a perda e atualizou a taxa de entrega.

## Parâmetros LoRa atuais

| Parâmetro | Valor |
| --- | --- |
| Frequência | 433 MHz |
| Bandwidth | 125 kHz |
| Spreading Factor | SF7 |
| Coding Rate | 4/5 |
| Preâmbulo | 8 símbolos |
| Header | Explícito |
| CRC | Ativado |
| Sync Word | `0x12` |
| Potência configurada na STM32 | Aproximadamente 12 dBm |
| Intervalo entre pacotes | Aproximadamente 2 segundos |

## Progresso

### Concluído

- [x] Preparação do Rust e instalação do alvo `thumbv7em-none-eabihf`.
- [x] Configuração do `probe-rs` e do ST-LINK.
- [x] Compilação e gravação do firmware na Nucleo-F446RE.
- [x] Validação inicial da STM32 com o LED LD2 e SysTick.
- [x] Ajuste do alinhamento da seção de código em `memory.x`.
- [x] Preparação do projeto `esp32-gateway/` com ESP-IDF.
- [x] Comunicação SPI entre ESP32 e SX1278.
- [x] Leitura do `RegVersion = 0x12` no ESP32.
- [x] Conexão do segundo SX1278 à STM32.
- [x] Comunicação SPI entre STM32 e SX1278 em Rust.
- [x] Leitura do `RegVersion = 0x12` na STM32.
- [x] Configuração compatível dos dois rádios em 433 MHz.
- [x] Primeira transmissão LoRa STM32 → ESP32.
- [x] Confirmação de transmissão `TxDone` na STM32.
- [x] Inclusão de número sequencial nos pacotes.
- [x] Coleta de RSSI e SNR no ESP32.
- [x] Contagem de pacotes recebidos, perdidos, duplicados e fora de ordem.
- [x] Cálculo da taxa de entrega.
- [x] Teste controlado de detecção de perda de pacote.

### Próximos passos

- [ ] Implementar confirmação de recebimento (`ACK`).
- [ ] Tornar a comunicação bidirecional.
- [ ] Medir latência ou tempo de ida e volta.
- [ ] Definir sessões de teste com início e término controlados.
- [ ] Enviar os resultados do ESP32 ao computador em formato estruturado.
- [ ] Salvar os resultados em CSV.
- [ ] Desenvolver análise e gráficos com Python.
- [ ] Comparar sessões variando distância, obstáculos e parâmetros LoRa.
- [ ] Documentar os testes e resultados no repositório.

## Marcos do projeto

| Etapa | Situação |
| --- | --- |
| STM32 executando firmware em Rust | Concluída |
| ESP32 executando firmware em C/ESP-IDF | Concluída |
| Comunicação local ESP32 + SX1278 | Concluída em 02/09/2026 |
| Comunicação local STM32 + SX1278 | Concluída |
| Primeira comunicação LoRa STM32 → ESP32 | Concluída em 05/09/2026 |
| Pacotes com número sequencial | Concluída em 05/09/2026 |
| RSSI, SNR e estatísticas de entrega | Concluída em 05/09/2026 |
| Comunicação bidirecional e ACK | Próxima etapa |
| Coleta e análise em Python | Planejada |

## Estrutura do repositório

| Caminho | Conteúdo |
| --- | --- |
| `stm32-blink/stm32-blink/` | Firmware da Nucleo-F446RE em Rust |
| `esp32-hello/` | Exemplo inicial utilizado para validar o ESP32 |
| `esp32-gateway/` | Firmware do gateway ESP32 e receptor LoRa |
| `README.md` | Documentação geral do projeto |

A pasta dupla `stm32-blink/stm32-blink/` corresponde à estrutura atual do projeto.

## Conexões STM32 — SX1278

| SX1278 | STM32 Nucleo-F446RE |
| --- | --- |
| VCC | 3V3 |
| GND | GND |
| SCK | D13 / PA5 |
| MISO | D12 / PA6 |
| MOSI | D11 / PA7 |
| NSS | D10 / PB6 |
| RESET | D9 / PC7 |
| DIO0 | D2 / PA10 |

Foi utilizado um capacitor de **10 µF** entre 3V3 e GND, próximo ao módulo LoRa.

> O pino PA5 também está conectado ao LED verde LD2 da Nucleo. Durante a comunicação SPI, ele é utilizado como SCK e não fica disponível para controlar o LED.

## Conexões ESP32 — SX1278

| SX1278 | ESP32 |
| --- | --- |
| VCC | 3V3 |
| GND | GND |
| SCK | GPIO18 |
| MISO | GPIO19 |
| MOSI | GPIO23 |
| NSS | GPIO5 |
| RESET | GPIO14 |
| DIO0 | GPIO26 |

## Firmware STM32

O firmware utiliza `stm32f4xx-hal 0.23.0`, SPI1, clock interno HSI, temporização via SysTick e logs RTT com `defmt`.

No Ubuntu/WSL:

```bash
cd ~/projetos/Projeto-Lora-Stm/stm32-blink/stm32-blink
cargo build
cargo run
```

A Nucleo deve estar conectada pela porta USB do ST-LINK e disponibilizada ao WSL. O comando `cargo run` compila, grava e executa o firmware usando `probe-rs`.

O firmware:

- reseta e identifica o SX1278;
- configura o rádio em 433 MHz;
- escreve o pacote no FIFO;
- inicia a transmissão;
- confirma o evento `TxDone`;
- transmite uma nova sequência aproximadamente a cada dois segundos.

| Arquivo | Finalidade |
| --- | --- |
| `Cargo.toml` | Dependências e perfis de compilação |
| `.cargo/config.toml` | Alvo ARM, linker e comando de gravação |
| `memory.x` | Mapa de memória do STM32 |
| `build.rs` | Disponibiliza o mapa de memória ao linker |
| `src/main.rs` | Firmware do transmissor LoRa |

## Firmware ESP32

O projeto `esp32-gateway/` utiliza C com ESP-IDF. O firmware configura o SX1278 como receptor contínuo e apresenta os pacotes e métricas no monitor serial.

No Ubuntu/WSL:

```bash
cd ~/projetos/Projeto-Lora-Stm/esp32-gateway
source ~/esp/esp-idf/export.sh
idf.py build
idf.py -p /dev/ttyUSB0 flash monitor
```

Na primeira configuração de uma nova cópia do projeto, execute:

```bash
idf.py set-target esp32
```

Para encerrar o monitor serial, pressione **Ctrl + ]**.

## Ambiente validado

| Ferramenta | Versão |
| --- | --- |
| Rust | 1.98.0 |
| Cargo | 1.98.0 |
| probe-rs | 0.32.0 |
| ESP-IDF | 6.0.1 |
| stm32f4xx-hal | 0.23.0 |

O firmware do ESP32 é desenvolvido no Ubuntu/WSL. O firmware STM32 também foi compilado e executado pelo WSL com o ST-LINK disponibilizado por USB.

O repositório de trabalho está localizado em:

```text
~/projetos/Projeto-Lora-Stm
```

As pastas `target/`, `build/` e `managed_components/`, assim como ambientes virtuais e caches, permanecem fora do versionamento conforme o `.gitignore`.

## Aprendizados envolvidos

- Rust embarcado e desenvolvimento `no_std`;
- C com ESP-IDF;
- GPIO e temporização;
- SPI;
- registradores e FIFO do SX1278;
- interrupções `TxDone` e `RxDone`;
- rádio LoRa ponto a ponto;
- RSSI e SNR;
- numeração sequencial de pacotes;
- detecção de perdas e cálculo da taxa de entrega;
- depuração de conexões físicas e comunicação serial.

## Autor

Guilherme Costa
