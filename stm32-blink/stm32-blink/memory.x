MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 512K
  RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}

/* Alinha o inicio do codigo em um multiplo de 8 bytes. */
_stext = ALIGN(ADDR(.vector_table) + SIZEOF(.vector_table), 8);