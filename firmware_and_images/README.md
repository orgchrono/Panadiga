# Firmware Dumps and Binary Images (`firmware_and_images/`)

Este diretório armazena os dumps brutos de memória e imagens de armazenamento do Panasonic DMR-EH55 utilizados pelas ferramentas de análise forense e engenharia reversa do `panadiga`.

> [!NOTE]
> Por motivos de restrições de tamanho do GitHub (arquivos > 100 MB) e proteção de propriedade intelectual/direitos autorais de terceiros, os dumps brutos proprietários não são versionados diretamente no repositório Git público.
> Ao trabalhar com o gravador, coloque seus dumps obtidos em bancada nesta pasta com os nomes descritos abaixo.

---

## 📁 Arquivos Esperados

### 1. Dump de Referência do HDD — Internet Archive (`panasonic DMR EH-55 Firmware hdd_100.bin`)
* **Descrição:** Dump bruto dos primeiros 100 MB de um HDD de Panasonic DMR-EH55 obtido do **Internet Archive** (**não** foi extraído do HDD físico da bancada). Contém o cabeçalho `MEIHDFS1.0`, tabelas espelho, journal transacional e duas partições de firmware cifradas via Feistel 64-bit ECB:
  * `extracted_partitions/hdd_partition_firmware_0x02000000.bin` (`6.173.264 bytes`, SHA-256 `6ec63de059c00b287a0e8a1a4102a8007bbccf3210a552bfb45118a03b85c898`) → descompacta (`--mode main`) para **`EH55_MAIN_3410PQ.bin`** (`13.003.052 bytes`, SHA-256 `c3275d39549b68f48db16bc5fd043c6f8a373d4ba416826de09be5c55dc991b1`, `tag='MAIN'`, `dst=0x57000000`).
  * `extracted_partitions/hdd_partition_firmware_0x04000000.bin` (`720.896 bytes`, SHA-256 `487d5951d4aa7bc45e5f634c4ca079f369db06d5cb62e41a173575cecd68dbae`) → descompacta (`--mode drv`) para **`EH55_DRV_S211.bin`** (`1.441.792 bytes`, SHA-256 `6f2b61ede80ea9e3e9185d3d677813593f3d361446678e437c905a0037060243`, `tag='DRV '`, `dst=0x90202000`).
* **Tamanho exato:** `104.857.600 bytes` (204.800 setores LBA de 512 bytes).
* **Hash MD5:** `ef4453fc8651903b0a4b0c1873d950d4`
* **Hash SHA-256:** `9d292d5b46f190775f359e30af11d074021fb321f040bf701d0ecf0f2056db00`

### 2. Contêiner Óptico de Atualização (`PANA_DVD.FRM`)
* **Descrição:** Pacote oficial de atualização de firmware via CD para o Panasonic DMR-EH55 (extraído de `UPDATE_EH55.zip`, modelo/variante `NP-309`), contendo 3 módulos cifrados na TOC de 48 bytes (`extracted_partitions/frm_unpacked/`):
  * `drv_s211_unpacked.bin` = **`EH55_DRV_S211.bin`** (`1.441.792 bytes`, SHA-256 `6f2b61ede80ea9e3e9185d3d677813593f3d361446678e437c905a0037060243`, idêntico byte a byte ao módulo da partição `0x04000000`).
  * `prog061_dec.bin` = **`eh55_frm_PROG061_BOOT.bin`** (`262.144 bytes`, SHA-256 `c7da9121174fcf4c54f35f5bb1f7b23a3dc0c93ab227158ad4fdd90a1ea24a81`, base VA `0x50400000`).
  * `main3060np_unpacked.bin` (`13.427.852 bytes`, SHA-256 `075f552ed583ba1835c0d44d1b533ff5a016b8d81d15d7166240098461ddcca7`, base VA `0x57000000`).
* **Tamanho exato:** `7.531.744 bytes`.
* **Hash SHA-256:** `a596d52a8220a9113777c9caf271ac4d518202d0639d6118a4d0bc03b9a6367f`

### 3. Dump da Flash NOR TSOP-48 (`IC5001.bin`, quando extraído em bancada)
* **Descrição:** Espaço reservado para futura leitura física de 8 MB da memória NOR Flash TSOP-48 da Digital PCB (nenhum dump físico de Flash ou HDD da placa em bancada foi obtido ainda; da placa real existem apenas capturas UART em `COM10` / `P7501`).
* **Tamanho esperado:** `8.388.608 bytes`.

### 4. Pacote de Serviço `DMR-EH58` (`dmr-eh58/DVDRAM.7z`)
* **Descrição:** Arquivo de serviço comparativo da mesma família MN103S DIGA (`BOOT_VER,0.63`, `DEV_VER,3110PC`):
  * `BOOT.BIN` → **`BOOT.BIN`** (`262.144 bytes`, SHA-256 `7c13c72a54c77387f3f7df0ddc213e4800e20b27eecbfd225cbc05a8fc73e377`, base VA `0x44000000`, `Loader ver0.63 [07/10/24]` em plaintext).
  * `DRV.BIN` (`720.896 bytes`, SHA-256 `3a9699dd576e96ac89d163094786f2c292bb68826bac344837ec3c9f9a0eccbf`) → descompactado (`--mode plain`) para **`DRV_decomp.bin`** (`1.441.792 bytes`, SHA-256 `694a4ab8aa29fcc45350dc0712047663602bdee7e4bff313b8ae62b057efd052`, base VA `0x40000000`, `MATSHITA R6-MV4 A0 S415`).
