# E-Chess Electronics

This contains the schematic for the chess board.  
It is designed for an ESP32-S3 development board.  

Currently it is wired completely manually. Therefore it does not contain a PCB yet.

## LEDs

The schematic contains one single LED as placeholder for all 64 WS2812 LEDs.  
They are routed in a snake pattern from the top right to the bottom right.
```
┌💡─💡─💡─💡─💡─💡─💡─💡 <- START
└💡─💡─💡─💡─💡─💡─💡─💡┐
┌💡─💡─💡─💡─💡─💡─💡─💡┘
└💡─💡─💡─💡─💡─💡─💡─💡┐
┌💡─💡─💡─💡─💡─💡─💡─💡┘
└💡─💡─💡─💡─💡─💡─💡─💡┐
┌💡─💡─💡─💡─💡─💡─💡─💡┘
└💡─💡─💡─💡─💡─💡─💡─💡 -> END
``` 

## Reed sensors

For the Reed sensors a matrix with simple reed contacts is used.  
It resembles the matrix described [here](https://www-user.tu-chemnitz.de/~heha/Mikrocontroller/Tastenmatrix.htm).

The IO expander MCP23017 is used to connect the columns and rows.  
As it has exactly 16 pins, it matches exactly the required 8x8 matrix.

The MCP23017 has built in pull-up resistors, so no external resistors are needed.

GPB (pin 0 - 7) is used as output for the columns.  
GPA (pin 21 - 28) is used as input for the rows.  

The matrix resembles the actual chess board starting with A1 at the bottom left.  
So A1 == GPB0|GPA0, A2 == GPB1|GPA0, ... H8 == GPB7|GPA7.

To read the status, the firmware will set all rows but one to high and then read the columns.  
Then it will do the same for the next row.  

## Power supply

The ESP32-S3 is powered by the USB-C connector.  

* Since the esp works with 3.3V, the IO expander is also running on 3.3V.  
* Since the LEDs are WS2812B, they need 5V, coming from the ESP dev board. 
 **You may need to connect the IN-OUT jumper on the ESP dev board to receive 5V on the 5V-pin.**

For now my plan is to avoid driving the LEDS with full power.  
I try to handle everything with 500 mA. However this may need to be changed later.