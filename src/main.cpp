#include <Arduino.h>

bool ledB2 = false;
bool ledB1 = false;
bool ledA2 = false;
bool ledA1 = false;


void setup() {
  Serial.begin(9600);

  // Grid input
  pinMode(4, INPUT_PULLUP);
  pinMode(5, INPUT_PULLUP);

  // Grid output
  pinMode(2, OUTPUT);
  pinMode(3, OUTPUT);

  // Leds
  pinMode(7, OUTPUT); // A2
  pinMode(8, OUTPUT); // B2
  pinMode(9, OUTPUT); // A1
  pinMode(10, OUTPUT); // B1
}

void loop() {
  ledB2 = false;
  ledB1 = false;
  ledA2 = false;
  ledA1 = false;

  // enable line 1
  digitalWrite(2, true);
  digitalWrite(3, false);
  //delay(100);

  if (!digitalRead(4)) {
    ledA2 = true;
  }

  if (!digitalRead(5)) {
    ledB2 = true;
  }
  Serial.println("ledA2 " + String(ledA2));
  Serial.println("ledB2 " + String(ledB2));

  // enable line 2
  digitalWrite(2, false);
  digitalWrite(3, true);
  //delay(100);

  if (!digitalRead(4)) {
    ledA1 = true;
  }

  if (!digitalRead(5)) {
    ledB1 = true;
  }
  Serial.println("ledA1 " + String(ledA1));
  Serial.println("ledB1 " + String(ledB1));
  Serial.println("________");
  

  digitalWrite(7, ledA2);
  digitalWrite(8, ledB2);
  digitalWrite(9, ledA1);
  digitalWrite(10, ledB1);

  delay(100);
}