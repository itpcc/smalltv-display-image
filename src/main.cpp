#include <Arduino.h>
#include <SPI.h>
#include <ESP8266WiFi.h>
#include <base64.h>
#include <LittleFS.h>
#include <ESP8266HTTPClient.h>
#include <WiFiClient.h>
#include <Ticker.h>

#include <TFT_eSPI.h> // Hardware-specific library

#include "bitmap.hpp"
#include "credentials.hpp"
#define CHECK_INTERVAL_MS 5000
#define DISPLAY_TIME_MS (CHECK_INTERVAL_MS * 6)

// Config is in `platformio.ini` build flags
// @see https://github.com/Bodmer/TFT_eSPI/blob/master/docs/PlatformIO/Configuring%20options.txt

TFT_eSPI tft = TFT_eSPI();  // Create object "tft"
Ticker msgChkTick;
WiFiClient client {};
HTTPClient http {};
unsigned long lastDspTs = 0;

void turnOnTFT() {
  tft.writecommand(TFT_DISPON);
  digitalWrite(TFT_BL, TFT_BACKLIGHT_ON);
}

void initWiFi() {
  WiFi.mode(WIFI_STA);
  WiFi.begin(WIFI_SSID, WIFI_PASSWORD);

  turnOnTFT();
  tft.fillScreen(TFT_DARKGREY);

  uint8_t attempt = 1;
  while(attempt > 0) {
    if ((attempt % 10) == 1) {
      tft.setCursor(10, 10);
      tft.print("Connecting to: ");
      tft.print(WIFI_SSID);
      tft.print(' ');
    }

    auto wifiStatus = WiFi.waitForConnectResult();
    switch (wifiStatus){
      case WL_CONNECTED:
        attempt = 0;
      break;
      case WL_NO_SSID_AVAIL:
        tft.print(" Not found");
        break;
      case WL_CONNECT_FAILED:
        tft.print(" Failed");
        break;
      case WL_WRONG_PASSWORD:
        tft.print(" Wrong P/W");
        break;
      break;
      default:
        tft.print('.');
      break;
    }

    if (attempt == 0) break;

    attempt = (attempt % 10) + 1;

    delay(1000);
  }

  // Connected
  tft.setCursor(10, 10);
  tft.print("Connected to: ");
  tft.print(WIFI_SSID);
  tft.setCursor(10, 26);
  tft.print("IP:");
  tft.print(WiFi.localIP().toString());
  tft.setCursor(10, 42);
  tft.print("RSSI:");
  tft.print(WiFi.RSSI(), 10);

  WiFi.setAutoReconnect(true);
  WiFi.persistent(true);
}

int fetchQR() {
  String const urlPrefix {SERVICE_HOST};
  uint8_t attempt = 0;

  // * Check latest timestamp
  http.begin(client, String {SERVICE_HOST}, SERVICE_PORT, "/latest_version", false);
  // @see https://github.com/esp8266/Arduino/issues/7688#issuecomment-752090992
  http.setAuthorization("");
  http.addHeader("Authorization", "Bearer " + base64::encode(SERVICE_TOKEN));

  while (attempt <= 3) {
    int const httpCode = http.GET();
    ++attempt;

    if (httpCode == HTTP_CODE_OK) break;
    if (attempt >= 3) return httpCode;

    delay(1000);
  }

  auto latestVersionStr = http.getString();
  latestVersionStr.trim();
  {
    File lastVersionFile = LittleFS.open("latest_version", "r");

    if (lastVersionFile.readString() == latestVersionStr) {
      http.end();
      return HTTP_CODE_NOT_MODIFIED;
    }

    lastVersionFile.close();
  };

  // Fetch the actual QR
  attempt = 0;
  String urlImg {"/qr-"};
  urlImg.concat(latestVersionStr);
  urlImg.concat(".bmp");
  http.setURL(urlImg);

  while (attempt <= 3) {
    int const httpCode = http.GET();
    ++attempt;

    if (httpCode == HTTP_CODE_OK) break;
    if (attempt >= 3) return httpCode;

    delay(1000);
  }

  {
    File qrFile = LittleFS.open("qr.bmp", "w");
    http.writeToStream(&qrFile);
    qrFile.close();
  };

  File lastUpdateFile = LittleFS.open("latest_version", "w");
  lastUpdateFile.print(latestVersionStr);
  lastUpdateFile.close();
  http.end();

  return HTTP_CODE_OK;
}

void checkNewMessage() {
  if (WiFi.status() != WL_CONNECTED) {
    turnOnTFT();
    tft.fillRectHGradient(0, 0, TFT_WIDTH, TFT_HEIGHT, TFT_OLIVE, TFT_YELLOW);
    tft.setCursor(10,10);
    tft.print("WIFI Status: ");
    tft.print((int) WiFi.status(), 10);
    delay(5000);
    WiFi.disconnect(false);
    initWiFi();
  }

  int const fetchRes = fetchQR();

  if (fetchRes == (int) HTTP_CODE_NOT_MODIFIED) {
    if ((unsigned long) millis() - lastDspTs > DISPLAY_TIME_MS) {
      tft.fillScreen(TFT_DARKGREY);
      tft.setCursor(10, 10);
      tft.print("Turning off...");
      delay(1000);
      tft.writecommand(TFT_DISPOFF);
      // XOR to reverse on to off. and mask to only 0 or 1
      digitalWrite(TFT_BL, (TFT_BACKLIGHT_ON ^ 1) & 1);
    }

    return;
  }

  String errMsg {};

  if (fetchRes == HTTP_CODE_OK) {
    errMsg = drawBmp(tft, "qr.bmp", 0, 0);

    if (errMsg.isEmpty()) {
      turnOnTFT();
      lastDspTs = (unsigned long) millis();
      return;
    }
  } else {
    errMsg = "Response Error: " + String(fetchRes, 10);
  }

  // Display error
  turnOnTFT();
  tft.fillRectHGradient(0, 0, TFT_WIDTH, TFT_HEIGHT, TFT_MAGENTA, TFT_RED);
  tft.setCursor(10,10);
  tft.print(errMsg);
}

void setup() {
  Serial.begin(115200);
  Serial.println("Hello setup");
  tft.init();
  tft.setRotation(0);
  tft.setTextFont(2);

  turnOnTFT();
  initWiFi();

  http.setTimeout(5000);
  http.setFollowRedirects(HTTPC_STRICT_FOLLOW_REDIRECTS);
  http.setReuse(true);
  LittleFS.begin();

  delay(1000);
}

void loop() {
  checkNewMessage();

  delay(CHECK_INTERVAL_MS);
}
