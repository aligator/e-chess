#include <Arduino.h>
#include <Adafruit_NeoPixel.h>

#define LED_PIN    8

// How many NeoPixels are attached to the Arduino?
#define LED_COUNT 9

// Declare our NeoPixel strip object:
Adafruit_NeoPixel strip(LED_COUNT, LED_PIN, NEO_GRB + NEO_KHZ800);
// Argument 1 = Number of pixels in NeoPixel strip
// Argument 2 = Arduino pin number (most are valid)
// Argument 3 = Pixel type flags, add together as needed:
//   NEO_KHZ800  800 KHz bitstream (most NeoPixel products w/WS2812 LEDs)
//   NEO_KHZ400  400 KHz (classic 'v1' (not v2) FLORA pixels, WS2811 drivers)
//   NEO_GRB     Pixels are wired for GRB bitstream (most NeoPixel products)
//   NEO_RGB     Pixels are wired for RGB bitstream (v1 FLORA pixels, not v2)
//   NEO_RGBW    Pixels are wired for RGBW bitstream (NeoPixel RGBW products)

const uint8_t FIELD_SIZE = 3;

const uint8_t COLS[FIELD_SIZE] = { 5, 6, 7 };
const uint8_t ROWS[FIELD_SIZE] = { 2, 3, 4 };

uint8_t field[FIELD_SIZE][FIELD_SIZE] = {
  {0, 0, 0},
  {0, 0, 0},
  {0, 0, 0},
};

void printField() {
  for (size_t row = 0; row < FIELD_SIZE; row++)
  {
    for (size_t col = 0; col < FIELD_SIZE; col++)
    {
      Serial.print(field[row][col]);
      field[row][col] = 0;
    }
    Serial.println("");
  }
}

void setup() {
  Serial.begin(9600);

  strip.begin();           // INITIALIZE NeoPixel strip object (REQUIRED)
  strip.show();            // Turn OFF all pixels ASAP
  strip.setBrightness(255); // Set BRIGHTNESS to about 1/5 (max = 255)

  // Grid input
  for (auto &&i : ROWS)
  {
    pinMode(i, INPUT_PULLUP);
  }
  
  // Grid output
  for (auto &&i : COLS)
  {
    pinMode(i, OUTPUT);
  }

  printField();


  for (size_t col = 0; col < FIELD_SIZE; col++)
  {
    digitalWrite(COLS[col], true);
  }
}

void loop() {
  // Check each field
  for (size_t col = 0; col < FIELD_SIZE; col++)
  {
    digitalWrite(COLS[col], false);
    
    for (size_t row = 0; row < FIELD_SIZE; row++)
    {
   
      field[row][col] = !digitalRead(ROWS[row]);

      uint16_t pixel = row*FIELD_SIZE+col;
      if (row % 2 == 0) {
        pixel = row*FIELD_SIZE+(FIELD_SIZE-col-1);
      }
      

      strip.setPixelColor(pixel, 100 * field[row][col], 0, 0);
    }
    digitalWrite(COLS[col], true);
  }
  
  strip.show();
}