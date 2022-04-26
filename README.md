# Air Quality (PM2.5) indoor sensor

To monitor indoor air quality during upcoming California Fire Season (now year-round!)

### Parts

* Raspberry Pi Zero W
* Plantower PM2.5 sensor from AdaFruit

**Following**: [Guide](https://learn.adafruit.com/pm25-air-quality-sensor)

[Datasheet](https://cdn-shop.adafruit.com/product-files/3686/plantower-pms5003-manual_v2-3.pdf)

[Hookup Diagram](https://learn.adafruit.com/assets/83709)

## Pi Setup

* follow [NuPiWhoDis](https://github.com/scottrfrancis/nuPiWhoDis) scripts
* **_CHANGE PASSWORD_**
* Ensure serial port is enabled and NOT for login shell

* install pip

```bash
sudo apt-get install python3-pip -y

# picocom, tmux, and vim also good
sudo apt install tmux vim picocom htop -y
```

* install requirements

```bash
pip3 install -r requirements.txt
```

## hookup

3 wires - +5V, GND, PM2.5 TX -> Pi0W S0 RX

## test output

```bash
stty -F /dev/ttyS0 9600


pi@airq:~ $ cat /dev/ttyS0 | xxd
00000000: 424d 001c 0003 0004 0004 0003 0004 0004  BM..............
00000010: 0216 00a6 001c 0002 0000 0000 9700 0234  ...............4
00000020: 424d 001c 0003 0004 0004 0003 0004 0004  BM..............
00000030: 0216 00a6 001c 0002 0000 0000 9700 0234  ...............4
00000040: 424d 001c 0003 0004 0004 0003 0004 0004  BM..............
00000050: 020a 009e 0020 0002 0000 0000 9700 0224  ..... .........$
00000060: 424d 001c 0003 0004 0004 0003 0004 0004  BM..............
00000070: 020a 009e 0020 0002 0000 0000 9700 0224  ..... .........$
00000080: 424d 001c 0003 0004 0004 0003 0004 0004  BM..............
00000090: 020a 009e 0020 0002 0000 0000 9700 0224  ..... .........$
000000a0: 424d 001c 0003 0004 0004 0003 0004 0004  BM..............
000000b0: 0216 00a2 0020 0002 0000 0000 9700 0234  ..... .........4
000000c0: 424d 001c 0003 0004 0004 0003 0004 0004  BM..............
000000d0: 0216 00a2 0020 0002 0000 0000 9700 0234  ..... .........4
```

From the datasheet:

* rate: 9600 bps
* payload length 32 bytes -- but organized as 16 16-bit quantities
* in default active mode, the device just sends repeatedly...

Framing:

See the [Datasheet](https://cdn-shop.adafruit.com/product-files/3686/plantower-pms5003-manual_v2-3.pdf) for details, but the frames are 16 unsigned 16-bit words. The first word is a flag sequnce 0x424D kind of ensures all the bit lanes are connected. The payload consists of 12 measures of various weightings and sizings. An odd reserved word 0x9700 (perhaps a version number?), and *byte-wise* checksum

## Building and testing the code

* checkout the repo
* `cargo test` will run all the tests
