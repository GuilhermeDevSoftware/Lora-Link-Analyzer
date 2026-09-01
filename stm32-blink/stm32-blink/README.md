# LoRa Link Analyzer — primeiro firmware STM32

Teste de bancada para NUCLEO-F446RE: LED verde LD2 (PA5) aceso por
aproximadamente 500 ms e apagado por aproximadamente 500 ms.
Utiliza Rust sem sistema operacional, stm32f4xx-hal 0.23.0 e clock interno HSI.

## Executar no Windows

Conecte a Nucleo pelo USB do ST-LINK e mantenha os radios desconectados.
Abra um terminal do Windows dentro desta pasta, onde esta Cargo.toml.

```powershell
rustup target add thumbv7em-none-eabihf
cargo build
cargo run
```

Execute cargo run apenas se cargo build terminar sem erros. O comando grava
este firmware na placa, substituindo a aplicacao anterior, e inicia sua execucao.
O terminal pode continuar ocupado acompanhando a placa; este exemplo nao imprime
mensagens e nao utiliza serial nem RTT. Para encerrar a sessao, pressione Ctrl+C;
pressione depois RESET na placa para reiniciar o firmware autonomamente.

## Arquivos

| Arquivo | Finalidade |
| --- | --- |
| Cargo.toml | Dependencias e configuracao de compilacao |
| .cargo/config.toml | Alvo ARM, linker e comando de gravacao |
| memory.x | Mapa de Flash e RAM |
| build.rs | Disponibiliza memory.x ao linker |
| src/main.rs | Configuracao do clock, GPIO e laco do LED |

Cargo.lock sera gerado no primeiro build. Guarde-o no Git junto com o projeto
para registrar as versoes resolvidas. A pasta target contem arquivos gerados.

## Entender e validar

- no_std: usa core, sem a biblioteca padrao de um sistema operacional.
- no_main e entry: entrada do firmware pelo runtime Cortex-M.
- HAL: fornece operacoes de configuracao de clock e controle dos perifericos.
- SysTick: temporizador do nucleo usado para os atrasos bloqueantes.
- loop: executa continuamente enquanto a placa estiver ligada.

1. Confirme o LED alternando entre aceso e apagado a cada meio segundo.
2. Troque os dois valores 500_u32 por 100_u32, salve e execute cargo run.
3. Confirme a mudanca visivel de velocidade: isso comprova a execucao do novo firmware.
4. Restaure 500_u32 e grave novamente se desejar manter o padrao inicial.

Status: arquivos e APIs revisados com documentacao oficial. Compilacao e teste
fisico ainda pendentes; este pacote nao inclui um binario pre-compilado.

## Referencias

- https://docs.rs/stm32f4xx-hal/0.23.0/stm32f4xx_hal/
- https://docs.rs/cortex-m/0.7.7/cortex_m/delay/struct.Delay.html
- https://www.st.com/resource/en/user_manual/um1724-stm32-nucleo64-boards-mb1136-stmicroelectronics.pdf
- https://www.st.com/en/microcontrollers-microprocessors/stm32f446re.html
- https://probe.rs/docs/tools/probe-rs/
