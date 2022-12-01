#!/bin/bash

stty -F /dev/ttyS0 9600 -parenb -parodd -cmspar cs8 \
    hupcl -cstopb cread clocal -crtscts \
    -ignbrk -brkint -ignpar -parmrk -inpck -istrip -inlcr -igncr \
    -icrnl -ixon -imaxbel -flusho  \
    -ixoff -iuclc -ixany -iutf8 \
    -opost -olcuc \
    -ocrnl onlcr -onocr -onlret -ofill -ofdel nl0 cr0 tab0 bs0 vt0 ff0 \
    -isig -icanon -iexten -echo echoe echok -echonl -noflsh -xcase \
    -tostop -echoprt echoctl echoke -extproc 

cd ~

echo "PM1,PM2.5,PM10"; while true ; do ./airq /dev/ttyS0; sleep 60; done
