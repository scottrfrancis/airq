---
runme:
  id: 01HWQS11Q220THE7VBMYA2GPJ0
  version: v3
---

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
* ___CHANGE PASSWORD___
* Ensure serial port is enabled and NOT for login shell
* install tools

```bash {"id":"01HWQS11Q220THE7VBMMJY5JFC"}
# picocom, tmux, and vim also good
sudo apt install tmux vim picocom htop -y
```

## hookup

3 wires - +5V, GND, PM2.5 TX -> Pi0W S0 RX

## test output

```bash {"id":"01HWQS11Q220THE7VBMRHACBJB"}
stty -F /dev/ttyS0 9600 -parenb -parodd -cmspar cs8 \
    hupcl -cstopb cread clocal -crtscts \
    -ignbrk -brkint -ignpar -parmrk -inpck -istrip -inlcr -igncr \
    -icrnl -ixon -imaxbel -flusho  \
    -ixoff -iuclc -ixany -iutf8 \
    -opost -olcuc \
    -ocrnl onlcr -onocr -onlret -ofill -ofdel nl0 cr0 tab0 bs0 vt0 ff0 \
    -isig -icanon -iexten -echo echoe echok -echonl -noflsh -xcase -tostop -echoprt echoctl echoke -extproc 

#  

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

See the [Datasheet](https://cdn-shop.adafruit.com/product-files/3686/plantower-pms5003-manual_v2-3.pdf) for details, but the frames are 16 unsigned 16-bit words. The first word is a flag sequnce 0x424D kind of ensures all the bit lanes are connected. The payload consists of 12 measures of various weightings and sizings. An odd reserved word 0x9700 (perhaps a version number?), and _byte-wise_ checksum

## Building and testing the code

* checkout the repo
* `cargo test` will run all the tests

## Extracting and reporting measurements

The sensor reports 12 measures per 32 byte frame at 9600 bps.
32*8 = 256 bits / 9600 bps = 26.67 mS/frame or about 37.5 frames of data per second.

That's pretty fast. Real indoor air quality isn't going to change that quickly. But, if we move the sensor from room to room (say to kitchen after cooking bacon), we'd want the sensor to respond fairly quickly.

As a starting point, let's average metrics over 100 frames (about 2.667 S) and write the readings out as CSV.

## Cross build for RPi 0W

follow - https://github.com/cross-rs/cross

then `cross build --target arm-unknown-linux-gnueabi`

transfer the binary (`target/arm-unknown-linux-gnueabi/debug/airq`) to the Pi.

## Loop the output with desired delay

The program is a ONE-SHOT that will read up to 64 bytes from the given file (e.g. `airq /dev/ttyS0`) until it finds a valid Payload struct.  If a struct is found, the PM1, PM2.5, and PM10 std concentrations will be be dumped -- separated by commas in case you want to build a CSV.

Since it is a one-shot, need to script the output.

```bash {"id":"01HWQS11Q220THE7VBMVMSP0GR"}
echo "PM1,PM2.5,PM10"; while true ; do ./airq /dev/ttyS0; sleep 60; done
```

## Modbus TCP Server

The three main readings (PM1.0, PM2.5, and PM10) are available on Modbus TCP Input registers 1..3. (NB, Modbus numbers registers from 1 not 0.)

The code in `main.rs` creates a modbus server on :5502 (all adapters - 0.0.0.0). To make this accessible to modbus integrations on the well-known port number, :502, an NFTables rule is needed.

See https://access.redhat.com/documentation/en-us/red_hat_enterprise_linux/7/html/security_guide/sec-configuring_port_forwarding_using_nftables

```bash {"id":"01HWTADEYS6B5CNSSQ086VC1MV"}
sudo su -

nft add table ip nat
nft -- add chain ip nat prerouting { type nat hook prerouting priority -100 \; }
nft add rule ip nat prerouting tcp dport 502 redirect to :5502
```

## Adding Temp/humidity with AM2302 (aka DHT 22 or 11)

Found some support for a Rust implementation - https://github.com/RougeEtoile/gpio-am2302-rs

It _works_ but isn't great.  I suppose it does help drive some error handling and robustness ;P

Since it does some timed, low-level bit banging with some unclear support and confused programming model, be sure to **ALWAYS BUILD RELEASE** to avoid any timing disruptions.

But the author published the crate with PRIVATE data fields for the reading, yet the github repo has those fields as public.  So... import by path.

## Summary of Modbus Registers

Note these are '0-based' addresses.  Some SCADA or other systems may be '1-based' and you may need to add 1 to the addresses.

| Input Register Address | Reading | Data Type | Comment |
| --- | --- | --- | --- | 
| 0x00 | AQI (computed) | Unsigned Integer (16-bit) | Computed from PM readings |
| 0x01 | PM1.0 | Unsigned Integer (16-bit) | PM 1.0 Reading |
| 0x02 | PM2.5 | Unsigned Integer (16-bit) | PM 2.5 |
| 0x03 | PM 10 | Unsigned Integer (16-bit) | PM 10 |
| 0x04 | AQI timestamp | Unsigned Long (32-bit) BE | 32-bit overflowing epoch seconds |
| 0x06 | Temp °C | Float (32-bit) BE | degrees Celsius |
| 0x08 | Humidity | Float (32-bit) BE | percentage |
| 0x0A | Temp timestamp | Unsigned Long (32-bit) BE | 32-bit overflowing epoch seconds |

Note that the word size for Modbus is 16-bits. Parameters requiring multiple register encodes are BIG ENDIAN encoded. Timestamps are the low-order 32 bits of the epoch timestamp and will overflow. While they should generally be monotonically increasing (other than rollover), they should not be used for precise timing, but rather to detect staleness or when Temp and AQI measures are significantly out of sync.

### Display

https://wiki.seeedstudio.com/Grove-LCD_RGB_Backlight/#resources

### AQI Calc

https://forum.airnowtech.org/t/the-aqi-equation/169
https://www.epa.gov/sites/default/files/2014-05/documents/zell-aqi.pdf

may need additional sensors for airborne chemicals

https://www.seeedstudio.com/Grove-Air-Quality-Sensor-v1-3-Arduino-Compatible.html

to Compute AQI, only use the first 3 metrics - Pariculate matter of 1.0, 2.5, and 10 microns in micro-grams/cubic meter.  AQI only uses 2.5 and 10 micron though.  The AQI calc also uses PM2.5 in 0.1 ug/m3... so need to multiply reading from sensor by 10.

the PM10 concentration is approx the same as the unitless AQI.  Multiply PM 2.5 by 3 for about the same scale.  PM1 doesn't have a standard, but could me most damaging... probaly muliply 9 or 10 for a rough approximation.

