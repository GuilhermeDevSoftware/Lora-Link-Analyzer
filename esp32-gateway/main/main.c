#include <stdio.h>

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "driver/gpio.h"
#include "driver/spi_master.h"

#include "esp_log.h"
#include "esp_err.h"

#define LORA_PIN_MISO      19
#define LORA_PIN_MOSI      23
#define LORA_PIN_SCK       18
#define LORA_PIN_NSS       5
#define LORA_PIN_RST       14
#define LORA_PIN_DIO0      26

#define SX1278_REG_VERSION 0x42

static const char *TAG = "SX1278";

static spi_device_handle_t lora_spi;

static void sx1278_reset(void)
{
    ESP_LOGI(TAG, "Resetando SX1278...");

    gpio_set_level(LORA_PIN_RST, 0);
    vTaskDelay(pdMS_TO_TICKS(10));

    gpio_set_level(LORA_PIN_RST, 1);
    vTaskDelay(pdMS_TO_TICKS(10));
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

    esp_err_t err = spi_device_transmit(
        lora_spi,
        &transaction
    );

    if (err != ESP_OK)
    {
        ESP_LOGE(
            TAG,
            "Erro na comunicacao SPI: %s",
            esp_err_to_name(err)
        );

        return 0;
    }

    return rx_data[1];
}

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
        .max_transfer_sz = 32
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

    ESP_LOGI(TAG, "SPI inicializado.");
}

void app_main(void)
{
    ESP_LOGI(TAG, "LoRa Link Analyzer");
    ESP_LOGI(TAG, "Teste SX1278");

    sx1278_gpio_init();
    sx1278_spi_init();
    sx1278_reset();

    uint8_t version =
        sx1278_read_register(SX1278_REG_VERSION);

    ESP_LOGI(
        TAG,
        "RegVersion = 0x%02X",
        version
    );

    if (version == 0x12)
    {
        ESP_LOGI(
            TAG,
            "SX1278 detectado com sucesso!"
        );
    }
    else
    {
        ESP_LOGE(
            TAG,
            "SX1278 nao detectado."
        );
    }

    while (1)
    {
        vTaskDelay(pdMS_TO_TICKS(1000));
    }
}
