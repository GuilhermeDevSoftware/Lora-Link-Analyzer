#include <stdio.h>
#include <stdint.h>
#include <string.h>

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "driver/gpio.h"
#include "driver/spi_master.h"

#include "esp_err.h"
#include "esp_log.h"

/*
 * Pinagem ESP32 ↔ SX1278
 */
#define LORA_PIN_MISO 19
#define LORA_PIN_MOSI 23
#define LORA_PIN_SCK  18
#define LORA_PIN_NSS  5
#define LORA_PIN_RST  14
#define LORA_PIN_DIO0 26

/*
 * Registradores do SX1278
 */
#define REG_FIFO                 0x00
#define REG_OP_MODE              0x01
#define REG_FRF_MSB              0x06
#define REG_FRF_MID              0x07
#define REG_FRF_LSB              0x08
#define REG_LNA                  0x0C
#define REG_FIFO_ADDR_PTR        0x0D
#define REG_FIFO_RX_BASE_ADDR    0x0F
#define REG_FIFO_RX_CURRENT_ADDR 0x10
#define REG_IRQ_FLAGS            0x12
#define REG_RX_NB_BYTES          0x13
#define REG_PKT_SNR_VALUE        0x19
#define REG_PKT_RSSI_VALUE       0x1A
#define REG_MODEM_CONFIG_1       0x1D
#define REG_MODEM_CONFIG_2       0x1E
#define REG_PREAMBLE_MSB         0x20
#define REG_PREAMBLE_LSB         0x21
#define REG_MODEM_CONFIG_3       0x26
#define REG_SYNC_WORD            0x39
#define REG_DIO_MAPPING_1        0x40
#define REG_VERSION              0x42

/*
 * Interrupções
 */
#define IRQ_RX_DONE           0x40
#define IRQ_PAYLOAD_CRC_ERROR 0x20

/*
 * Modos LoRa para 433 MHz
 */
#define MODE_LORA_SLEEP      0x88
#define MODE_LORA_STANDBY    0x89
#define MODE_LORA_RX_CONTINUOUS 0x8D

static const char *TAG = "LORA_RX";

static spi_device_handle_t lora_spi;

static void sx1278_gpio_init(void)
{
    gpio_config_t rst_config = {
        .pin_bit_mask = (1ULL << LORA_PIN_RST),
        .mode = GPIO_MODE_OUTPUT,
        .pull_up_en = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type = GPIO_INTR_DISABLE
    };

    ESP_ERROR_CHECK(gpio_config(&rst_config));

    gpio_config_t dio0_config = {
        .pin_bit_mask = (1ULL << LORA_PIN_DIO0),
        .mode = GPIO_MODE_INPUT,
        .pull_up_en = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type = GPIO_INTR_DISABLE
    };

    ESP_ERROR_CHECK(gpio_config(&dio0_config));
}

static void sx1278_spi_init(void)
{
    spi_bus_config_t bus_config = {
        .mosi_io_num = LORA_PIN_MOSI,
        .miso_io_num = LORA_PIN_MISO,
        .sclk_io_num = LORA_PIN_SCK,
        .quadwp_io_num = -1,
        .quadhd_io_num = -1,
        .max_transfer_sz = 256
    };

    ESP_ERROR_CHECK(
        spi_bus_initialize(
            SPI2_HOST,
            &bus_config,
            SPI_DMA_DISABLED
        )
    );

    spi_device_interface_config_t device_config = {
        .clock_speed_hz = 1000000,
        .mode = 0,
        .spics_io_num = LORA_PIN_NSS,
        .queue_size = 1
    };

    ESP_ERROR_CHECK(
        spi_bus_add_device(
            SPI2_HOST,
            &device_config,
            &lora_spi
        )
    );

    ESP_LOGI(TAG, "SPI inicializado");
}

static void sx1278_reset(void)
{
    ESP_LOGI(TAG, "Resetando SX1278");

    gpio_set_level(LORA_PIN_RST, 0);
    vTaskDelay(pdMS_TO_TICKS(10));

    gpio_set_level(LORA_PIN_RST, 1);
    vTaskDelay(pdMS_TO_TICKS(20));
}

static uint8_t sx1278_read_register(uint8_t address)
{
    uint8_t tx_data[2] = {
        address & 0x7F,
        0x00
    };

    uint8_t rx_data[2] = {0};

    spi_transaction_t transaction = {
        .length = 16,
        .tx_buffer = tx_data,
        .rx_buffer = rx_data
    };

    esp_err_t error = spi_device_transmit(
        lora_spi,
        &transaction
    );

    if (error != ESP_OK)
    {
        ESP_LOGE(
            TAG,
            "Erro de leitura SPI: %s",
            esp_err_to_name(error)
        );

        return 0;
    }

    return rx_data[1];
}

static void sx1278_write_register(
    uint8_t address,
    uint8_t value
)
{
    uint8_t tx_data[2] = {
        address | 0x80,
        value
    };

    spi_transaction_t transaction = {
        .length = 16,
        .tx_buffer = tx_data,
        .rx_buffer = NULL
    };

    ESP_ERROR_CHECK(
        spi_device_transmit(
            lora_spi,
            &transaction
        )
    );
}

static void sx1278_read_fifo(
    uint8_t *payload,
    uint8_t length
)
{
    uint8_t tx_data[256] = {0};
    uint8_t rx_data[256] = {0};

    tx_data[0] = REG_FIFO & 0x7F;

    spi_transaction_t transaction = {
        .length = (length + 1) * 8,
        .tx_buffer = tx_data,
        .rx_buffer = rx_data
    };

    ESP_ERROR_CHECK(
        spi_device_transmit(
            lora_spi,
            &transaction
        )
    );

    memcpy(payload, &rx_data[1], length);
}

static bool sx1278_detect(void)
{
    uint8_t version =
        sx1278_read_register(REG_VERSION);

    ESP_LOGI(
        TAG,
        "RegVersion = 0x%02X",
        version
    );

    return version == 0x12;
}

static void sx1278_configure_receiver(void)
{
    /*
     * LoRa + frequência baixa + Sleep.
     * O modo LoRa somente deve ser alterado durante Sleep.
     */
    sx1278_write_register(
        REG_OP_MODE,
        MODE_LORA_SLEEP
    );

    vTaskDelay(pdMS_TO_TICKS(10));

    /*
     * Frequência: 433 MHz
     * FRF = 0x6C4000
     */
    sx1278_write_register(REG_FRF_MSB, 0x6C);
    sx1278_write_register(REG_FRF_MID, 0x40);
    sx1278_write_register(REG_FRF_LSB, 0x00);

    /*
     * BW = 125 kHz
     * Coding Rate = 4/5
     * Header explícito
     */
    sx1278_write_register(
        REG_MODEM_CONFIG_1,
        0x72
    );

    /*
     * SF7
     * CRC ativado
     */
    sx1278_write_register(
        REG_MODEM_CONFIG_2,
        0x74
    );

    /*
     * AGC automático ativado
     */
    sx1278_write_register(
        REG_MODEM_CONFIG_3,
        0x04
    );

    /*
     * Preâmbulo de 8 símbolos
     */
    sx1278_write_register(
        REG_PREAMBLE_MSB,
        0x00
    );

    sx1278_write_register(
        REG_PREAMBLE_LSB,
        0x08
    );

    /*
     * Sync Word igual ao transmissor
     */
    sx1278_write_register(
        REG_SYNC_WORD,
        0x12
    );

    /*
     * Ganho automático do receptor.
     */
    uint8_t lna = sx1278_read_register(REG_LNA);

    sx1278_write_register(
        REG_LNA,
        lna | 0x03
    );

    /*
     * Área de recepção do FIFO começa em zero.
     */
    sx1278_write_register(
        REG_FIFO_RX_BASE_ADDR,
        0x00
    );

    sx1278_write_register(
        REG_FIFO_ADDR_PTR,
        0x00
    );

    /*
     * DIO0 = RxDone.
     * Bits 7:6 = 00.
     */
    sx1278_write_register(
        REG_DIO_MAPPING_1,
        0x00
    );

    /*
     * Limpa interrupções anteriores.
     */
    sx1278_write_register(
        REG_IRQ_FLAGS,
        0xFF
    );

    sx1278_write_register(
        REG_OP_MODE,
        MODE_LORA_STANDBY
    );

    vTaskDelay(pdMS_TO_TICKS(10));

    /*
     * Entra em recepção contínua.
     */
    sx1278_write_register(
        REG_OP_MODE,
        MODE_LORA_RX_CONTINUOUS
    );

    ESP_LOGI(TAG, "Frequencia: 433 MHz");
    ESP_LOGI(TAG, "Bandwidth: 125 kHz");
    ESP_LOGI(TAG, "Spreading Factor: SF7");
    ESP_LOGI(TAG, "Coding Rate: 4/5");
    ESP_LOGI(TAG, "CRC ativado");
    ESP_LOGI(TAG, "Receptor LoRa iniciado");
}

static void sx1278_receive_packet(void)
{
    uint8_t irq_flags =
        sx1278_read_register(REG_IRQ_FLAGS);

    /*
     * Ignora a chamada caso RxDone não esteja ativo.
     */
    if ((irq_flags & IRQ_RX_DONE) == 0)
    {
        return;
    }

    /*
     * Descarta pacotes com erro de CRC.
     */
    if ((irq_flags & IRQ_PAYLOAD_CRC_ERROR) != 0)
    {
        ESP_LOGW(
            TAG,
            "Pacote descartado: erro de CRC"
        );

        sx1278_write_register(
            REG_IRQ_FLAGS,
            0xFF
        );

        return;
    }

    uint8_t packet_length =
        sx1278_read_register(REG_RX_NB_BYTES);

    uint8_t current_address =
        sx1278_read_register(
            REG_FIFO_RX_CURRENT_ADDR
        );

    /*
     * Posiciona o ponteiro no começo do pacote recebido.
     */
    sx1278_write_register(
        REG_FIFO_ADDR_PTR,
        current_address
    );

    uint8_t payload[256] = {0};

    sx1278_read_fifo(
        payload,
        packet_length
    );

    /*
     * Garante terminação para mostrar como texto.
     */
    payload[packet_length] = '\0';

    int8_t snr_raw =
        (int8_t)sx1278_read_register(
            REG_PKT_SNR_VALUE
        );

    float snr_db = snr_raw / 4.0f;

    uint8_t rssi_raw =
        sx1278_read_register(
            REG_PKT_RSSI_VALUE
        );

    /*
     * Para a faixa de 433 MHz, o offset é -164 dBm.
     */
    float rssi_dbm = -164.0f + rssi_raw;

    if (snr_db < 0)
    {
        rssi_dbm += snr_db;
    }

    ESP_LOGI(TAG, "==============================");

    ESP_LOGI(
        TAG,
        "Pacote recebido: %s",
        (char *)payload
    );

    ESP_LOGI(
        TAG,
        "Tamanho: %u bytes",
        packet_length
    );

    ESP_LOGI(
        TAG,
        "RSSI: %.1f dBm",
        (double)rssi_dbm
    );

    ESP_LOGI(
        TAG,
        "SNR: %.2f dB",
        (double)snr_db
    );

    ESP_LOGI(TAG, "==============================");

    /*
     * Limpa RxDone e as demais interrupções.
     */
    sx1278_write_register(
        REG_IRQ_FLAGS,
        0xFF
    );
}

void app_main(void)
{
    ESP_LOGI(TAG, "LoRa Link Analyzer");
    ESP_LOGI(TAG, "ESP32 Gateway/Receptor");

    sx1278_gpio_init();
    sx1278_spi_init();
    sx1278_reset();

    if (!sx1278_detect())
    {
        ESP_LOGE(
            TAG,
            "SX1278 nao detectado"
        );

        while (1)
        {
            vTaskDelay(pdMS_TO_TICKS(1000));
        }
    }

    ESP_LOGI(
        TAG,
        "SX1278 detectado com sucesso"
    );

    sx1278_configure_receiver();

    while (1)
    {
        /*
         * DIO0 sobe quando um pacote completo é recebido.
         */
        if (gpio_get_level(LORA_PIN_DIO0))
        {
            sx1278_receive_packet();
        }

        vTaskDelay(pdMS_TO_TICKS(10));
    }
}