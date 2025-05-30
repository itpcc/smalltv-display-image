// Based on Bodmer's BMP image rendering function
#include <LittleFS.h>
#include <TFT_eSPI.h> // Hardware-specific library

// These read 16- and 32-bit types from the SD card file.
// BMP data is stored little-endian, Arduino is little-endian too.
// May need to reverse subscript order if porting elsewhere.

uint16_t read16(fs::File &f) {
	uint16_t result = 0;
	result |= ((uint16_t) f.read()); // LSB
	result |= ((uint16_t) f.read() << 8); // MSB
	return result;
}

uint32_t read32(fs::File &f) {
	uint32_t result = 0;
	result |= ((uint32_t) read16(f)); // LSB
	result |= ((uint32_t) read16(f) << 16); // MSB
	return result;
}

String drawBmp(TFT_eSPI &tft, String const & filename, int16_t x, int16_t y) {
	if ((x >= tft.width()) || (y >= tft.height())) return "Position excess";

	// Open requested file on SD card
	auto bmpFS = LittleFS.open(filename, "r");

	if (!bmpFS) return "File not found: " + filename;

	uint32_t seekOffset;
	uint16_t w, h, row;
	uint8_t  r, g, b;

	uint32_t startTime = millis();

	if (read16(bmpFS) != 0x4D42) {
		bmpFS.close();
		return "BMP Magic code not regcognized.";
	}

	read32(bmpFS);
	read32(bmpFS);
	seekOffset = read32(bmpFS);
	read32(bmpFS);
	w = read32(bmpFS);
	h = read32(bmpFS);

	if (!((read16(bmpFS) == 1) && (read16(bmpFS) == 24) && (read32(bmpFS) == 0))) {
		bmpFS.close();
		return "BMP format not recognized.";
	}

	y += h - 1;

	bool oldSwapBytes = tft.getSwapBytes();
	tft.setSwapBytes(true);
	bmpFS.seek(seekOffset);

	uint16_t padding = (4 - ((w * 3) & 3)) & 3;
	uint8_t lineBuffer[w * 3 + padding];

	for (row = 0; row < h; row++) {
		bmpFS.read(lineBuffer, sizeof(lineBuffer));
		uint8_t*  bptr = lineBuffer;
		uint16_t* tptr = (uint16_t*)lineBuffer;
		// Convert 24 to 16-bit colours
		for (uint16_t col = 0; col < w; col++) {
			b = *bptr++;
			g = *bptr++;
			r = *bptr++;
			*tptr++ = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
		}

		// Push the pixel row to screen, pushImage will crop the line if needed
		// y is decremented as the BMP image is drawn bottom up
		tft.pushImage(x, y--, w, 1, (uint16_t*)lineBuffer);
	}

	tft.setSwapBytes(oldSwapBytes);
	Serial.print("Loaded in "); Serial.print(millis() - startTime);
	Serial.println(" ms");

	bmpFS.close();
	return "";
}
