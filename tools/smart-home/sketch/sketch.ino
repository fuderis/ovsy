#include <IRremote.h>
#include <Servo.h>
#include <ArduinoJson.h>

//    PINS:
#define RECV_PIN 2
const uint8_t relay_pins[4] = { 7, 8, 14, 15 };
const uint8_t servo_pins[4] = { A0, A1, A2, A3 };
const uint8_t motor_pins[2][2] = { {9,10}, {5,6} };

//    BINDS:
#define BIND_COUNT 5
unsigned long rele1_binds[BIND_COUNT]  = {0xFF807F, 0x8AB3679B, 0x62CC10D9, 0, 0};
unsigned long rele2_binds[BIND_COUNT]  = {0, 0, 0, 0, 0};
unsigned long rele3_binds[BIND_COUNT]  = {0, 0, 0, 0, 0};
unsigned long rele4_binds[BIND_COUNT]  = {0, 0, 0, 0, 0};

unsigned long servo1_binds[BIND_COUNT] = {0, 0, 0, 0, 0};
unsigned long servo2_binds[BIND_COUNT] = {0, 0, 0, 0, 0};
unsigned long servo3_binds[BIND_COUNT] = {0, 0, 0, 0, 0};
unsigned long servo4_binds[BIND_COUNT] = {0, 0, 0, 0, 0};

unsigned long motor1_binds[BIND_COUNT] = {0, 0, 0, 0, 0};
unsigned long motor2_binds[BIND_COUNT] = {0, 0, 0, 0, 0};

unsigned long* binds_arr[] = {
  rele1_binds, rele2_binds, rele3_binds, rele4_binds,
  servo1_binds, servo2_binds, servo3_binds, servo4_binds,
  motor1_binds, motor2_binds
};

// Action states
bool relay_state[4] = {0,0,0,0};
bool servo_state[4] = {0,0,0,0}; // false=0°, true=180°
int8_t motor_state[2] = {0,0};   // 0=off, 1=right, -1=left

// IR setup
IRrecv irrecv(RECV_PIN); 
decode_results results;

// Servo setup
Servo servos[4];

// JSON setup
StaticJsonDocument<384> doc; // Enough for small arrays, increase if ever needed

void setup() {
  Serial.begin(115200);
  irrecv.enableIRIn();

  // Init relay pins:
  for (uint8_t i=0; i<4; i++) {
    pinMode(relay_pins[i], OUTPUT);
  }

  // Init servo pins:
  for (uint8_t i=0; i<4; i++) {
    servos[i].attach(servo_pins[i]);
    servos[i].write(0);
  }

  // Init motors pins:
  for (uint8_t i=0; i<2; i++) {
    pinMode(motor_pins[i][0], OUTPUT);
    pinMode(motor_pins[i][1], OUTPUT);
  }

  // PWM frequency for pins 9 and 10:
  TCCR1A = 0;
  TCCR1B = 0;
  TCCR1A = (1 << COM1A1) | (1 << COM1B1) | (1 << WGM11);
  TCCR1B = (1 << WGM13) | (1 << WGM12) | (1 << CS10);
  ICR1 = 799; // (16MHz / (1 * (799 + 1)) )
  OCR1A = 0;
  OCR1B = 0;
}

void loop() {
  handle_serial_input();

  if (irrecv.decode(&results)) {
    handle_ir_code(results.value);
    send_code_to_pc(results.value);
    irrecv.resume();
  }
}

// Analog write for pin 9
void analogWrite9(uint16_t duty) {
    OCR1A = constrain(duty, 0, 799);
}

// Analog write for pin 10
void analogWrite10(uint16_t duty) {
    OCR1B = constrain(duty, 0, 799);
}

// Receive JSON with new binds from PC, override defaults
void handle_serial_input() {
  if (Serial.available()) {
    String input = Serial.readStringUntil('\n');
    DeserializationError err = deserializeJson(doc, input);

    if (!err) {}
    else if (doc["type"] == String("check")) {
      Serial.println("true");
    }
    else if (doc["type"] == String("codes")) {
      set_binds_from_json(doc);
    }
  }
}

// Set binds from incoming JSON (format: ["0xFFA25D", ...])
void set_binds_from_json(const JsonDocument& doc) {
  const char* bind_names[] = {
    "rele1", "rele2", "rele3", "rele4",
    "servo1", "servo2", "servo3", "servo4",
    "motor1", "motor2"
  };

  for (uint8_t idx = 0; idx < 10; idx++) {
    JsonVariantConst arr_var = doc[bind_names[idx]];
    
    if (!arr_var.isNull() && arr_var.is<JsonArray>()) {
      JsonArrayConst arr = arr_var.as<JsonArrayConst>();

      for (uint8_t i = 0; i < BIND_COUNT; i++) {
        if (i < arr.size())
          binds_arr[idx][i] = strtoul(arr[i].as<const char*>(), 0, 16);
        else
          binds_arr[idx][i] = 0;
      }
    }
  }
}


// Check if code matches one of two binds
bool code_in_binds(unsigned long code, unsigned long* binds) {
  if (binds[0] != 0 && code == binds[0]) return true;
  if (binds[1] != 0 && code == binds[1]) return true;
  return false;
}

// Route IR code to appropriate action
void handle_ir_code(unsigned long code) {
  for (uint8_t idx = 0; idx < 4; idx++)
    if (code_in_binds(code, binds_arr[idx])) toggle_relay(idx);
  for (uint8_t idx = 0; idx < 4; idx++)
    if (code_in_binds(code, binds_arr[4 + idx])) toggle_servo(idx);
  for (uint8_t idx = 0; idx < 2; idx++)
    if (code_in_binds(code, binds_arr[8 + idx])) toggle_motor(idx);
}

// Toggle relay state
void toggle_relay(uint8_t n) {
  relay_state[n] = !relay_state[n];
  digitalWrite(relay_pins[n], relay_state[n]);
  send_relay_to_pc(n, relay_state[n]);
}

// Toggle servo between 0° and 180°
void toggle_servo(uint8_t n) {
  servo_state[n] = !servo_state[n];
  servos[n].write(servo_state[n] ? 180 : 0);
  send_servo_to_pc(n, servo_state[n] ? 180 : 0);
}

// Toggle motor direction: off → right → left → off
void toggle_motor(uint8_t n) {
  if (motor_state[n] == 0) {
    motor_state[n] = 1;
    start_motor(n, 1);
  } else if (motor_state[n] == 1) {
    motor_state[n] = -1;
    start_motor(n, -1);
  } else {
    motor_state[n] = 0;
    stop_motor(n);
  }
}

// Start motor, dir: 1=right, -1=left
void start_motor(uint8_t n, int8_t dir) {
  if (n == 0) {
    if (dir == 1) {
      analogWrite9(460);
      analogWrite10(0);
    } else {
      analogWrite9(0);
      analogWrite10(460);
    }
  } else {
    if (dir == 1) {
      analogWrite(motor_pins[n][0], 128);
      analogWrite(motor_pins[n][1], 0);
    } else {
      analogWrite(motor_pins[n][0], 0);
      analogWrite(motor_pins[n][1], 128);
    }
  }

  send_motor_to_pc(n, dir, 1);
}

// Stop motor
void stop_motor(uint8_t n) {
  if (n == 0) {
    analogWrite9(0);
    analogWrite10(0);
  } else {
    analogWrite(motor_pins[n][0], 0);
    analogWrite(motor_pins[n][1], 0);
  }

  send_motor_to_pc(n, motor_state[n], 0);
}


// --- PC feedback: ---

// Send raw IR code event
void send_code_to_pc(unsigned long code) {
  StaticJsonDocument<64> doc;
  doc["type"] = "code";

  char out[18];
  sprintf(out, "0x%lX", code);
  doc["code"] = out;

  doc["millis"] = millis();

  serializeJson(doc, Serial);
  Serial.println();
}

// Send relay state event
void send_relay_to_pc(uint8_t idx, bool st) {
  StaticJsonDocument<64> doc;
  doc["type"] = "rele";
  doc["idx"] = idx + 1;
  doc["enabled"] = st;

  serializeJson(doc, Serial);
  Serial.println();
}

// Send servo position event
void send_servo_to_pc(uint8_t idx, int deg) {
  StaticJsonDocument<64> doc;
  doc["type"] = "servo";
  doc["idx"] = idx + 1;
  doc["degs"] = deg;

  serializeJson(doc, Serial);
  Serial.println();
}

// Send motor state event
void send_motor_to_pc(uint8_t idx, int8_t dir, bool enabled) {
  StaticJsonDocument<64> doc;
  doc["type"] = "motor";
  doc["idx"] = idx + 1;
  doc["direct"] = dir == 1 ? "right" : "left";
  doc["enabled"] = enabled;

  serializeJson(doc, Serial);
  Serial.println();
}
