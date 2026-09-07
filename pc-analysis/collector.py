import csv
import math
from datetime import datetime, timezone
from pathlib import Path

import serial


PORT = "/dev/ttyUSB0"
BAUD_RATE = 115200

OUTPUT_DIR = Path(__file__).resolve().parent / "data"

COLUMNS = [
    "pc_received_at_utc",
    "sequence",
    "payload_bytes",
    "rssi_dbm",
    "snr_db",
    "is_duplicate",
    "received_unique",
    "sequence_gaps",
    "duplicates_total",
    "out_of_order_total",
]

def parse_packet(line):
    # Ignora os logs normais do ESP-IDF.
    if not line.startswith("LORA_DATA,"):
        return None

    fields = line.split(",")

    if len(fields) != 10:
        raise ValueError("Quantidade incorreta de campos")

    sequence = int(fields[1])
    payload_bytes = int(fields[2])
    rssi_dbm = float(fields[3])
    snr_db = float(fields[4])
    is_duplicate = int(fields[5])

    counters = [int(value) for value in fields[6:10]]

    if not 1 <= sequence <= 99999:
        raise ValueError("Sequência fora do intervalo esperado")

    if not 1 <= payload_bytes <= 255:
        raise ValueError("Tamanho de pacote inválido")

    if is_duplicate not in (0, 1):
        raise ValueError("Indicador de duplicata inválido")

    if any(value < 0 for value in counters):
        raise ValueError("Contador negativo")

    if not math.isfinite(rssi_dbm) or not math.isfinite(snr_db):
        raise ValueError("RSSI ou SNR inválido")

    return [
        sequence,
        payload_bytes,
        rssi_dbm,
        snr_db,
        is_duplicate,
        *counters,
    ]

def main():
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    session = datetime.now(timezone.utc).strftime(
        "%Y%m%d_%H%M%S_%f"
    )
    output_path = OUTPUT_DIR / f"lora_{session}.csv"

    saved_rows = 0

    try:
        # Configura antes de abrir para evitar ativar DTR/RTS
        # intencionalmente ao conectar.
        with serial.Serial(
            port=None,
            baudrate=BAUD_RATE,
            timeout=1,
            exclusive=True,
        ) as connection:
            connection.dtr = False
            connection.rts = False
            connection.port = PORT
            connection.open()

            with output_path.open(
                "x",
                newline="",
                encoding="utf-8",
            ) as file:
                writer = csv.writer(file)
                writer.writerow(COLUMNS)
                file.flush()

                print(f"Serial aberta: {PORT}")
                print(f"Arquivo: {output_path}")
                print("Coletando... Ctrl+C para encerrar.")

                pending = bytearray()

                while True:
                    # Pode retornar parte de uma linha quando há timeout.
                    chunk = connection.read_until(b"\n")

                    if not chunk:
                        continue

                    pending.extend(chunk)

                    # Proteção contra dados sem terminação de linha.
                    if len(pending) > 4096:
                        pending.clear()
                        print("Linha excessivamente longa descartada.")
                        continue

                    if not pending.endswith(b"\n"):
                        continue

                    line = pending.decode(
                        "utf-8",
                        errors="replace",
                    ).strip()
                    pending.clear()

                    try:
                        packet = parse_packet(line)
                    except ValueError as error:
                        print(f"Linha inválida: {error}")
                        continue

                    if packet is None:
                        continue

                    timestamp = datetime.now(
                        timezone.utc
                    ).isoformat(timespec="milliseconds")

                    writer.writerow([timestamp, *packet])
                    file.flush()
                    saved_rows += 1

                    print(
                        f"Salvos={saved_rows} | "
                        f"SEQ={packet[0]:05d} | "
                        f"RSSI={packet[2]:.1f} dBm | "
                        f"SNR={packet[3]:.2f} dB | "
                        f"Duplicata={packet[4]}"
                    )

    except KeyboardInterrupt:
        print("\nColeta encerrada pelo usuário.")

    except (serial.SerialException, OSError) as error:
        print(f"\nFalha na coleta: {error}")
        print("Confira a conexão USB, a porta e o acesso ao arquivo.")

    finally:
        print(f"Linhas de dados gravadas: {saved_rows}")


if __name__ == "__main__":
    main()
