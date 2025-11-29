#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;

	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if(pixs_payload == NULL) return 0;

	PIX *pix1, *pix2, *pix3, *pix4, *pix5, *return_pix1, *payload_copy;

	pix1 = pixRead("../test8.jpg");
	payload_copy = pixCopy(NULL, pixs_payload);
	pixBackgroundNormGrayArray(payload_copy, pix1, 10, 10, 10, 10, 256, 10, 10, &pix2);
	pixDestroy(&pix1);
	pixDestroy(&pix2);
	pixDestroy(&payload_copy);

	pix1 = pixRead("../test8.jpg");
	payload_copy = pixCopy(NULL, pixs_payload);
	pixBackgroundNormGrayArrayMorph(payload_copy, pix1, 6, 5, 256, &pix2);
	pixDestroy(&pix1);
	pixDestroy(&pix2);
	pixDestroy(&payload_copy);

	pix1 = pixRead("../test8.jpg");
	payload_copy = pixCopy(NULL, pixs_payload);
	return_pix1 = pixBackgroundNormMorph(payload_copy, pix1, 6, 5, 256);
	pixDestroy(&pix1);
	pixDestroy(&payload_copy);
	pixDestroy(&return_pix1);

	pix1 = pixRead("../test8.jpg");
	pix2 = pixRead("../test8.jpg");
	payload_copy = pixCopy(NULL, pixs_payload);
	pixBackgroundNormRGBArrays(payload_copy, pix1, pix2, 10, 10, 10, 10, 130, 10, 10, &pix3, &pix4, &pix5);
	pixDestroy(&pix1);
	pixDestroy(&pix2);
	pixDestroy(&pix3);
	pixDestroy(&pix4);
	pixDestroy(&pix5);
	pixDestroy(&payload_copy);

	pix1 = pixRead("../test8.jpg");
	payload_copy = pixCopy(NULL, pixs_payload);
	pixBackgroundNormRGBArraysMorph(payload_copy, pix1, 6, 33, 130, &pix2, &pix3, &pix4);
	pixDestroy(&pix1);
	pixDestroy(&pix2);
	pixDestroy(&pix3);
	pixDestroy(&pix4);
	pixDestroy(&payload_copy);

	payload_copy = pixCopy(NULL, pixs_payload);
	pixContrastNorm(payload_copy, payload_copy, 10, 10, 3, 0, 0);
	pixDestroy(&payload_copy);

	pix1 = pixRead("../test8.jpg");
	payload_copy = pixCopy(NULL, pixs_payload);
	return_pix1 = pixGlobalNormNoSatRGB(payload_copy, pix1, 3, 3, 3, 2, 0.9);
	pixDestroy(&pix1);
	pixDestroy(&payload_copy);
	pixDestroy(&return_pix1);

	payload_copy = pixCopy(NULL, pixs_payload);
	pixThresholdSpreadNorm(payload_copy, L_SOBEL_EDGE, 10, 0, 0, 0.7, -25, 255, 10, &pix1, &pix2, &pix3);
	pixDestroy(&pix1);
	pixDestroy(&pix2);
	pixDestroy(&pix3);
	pixDestroy(&payload_copy);

	pixDestroy(&pixs_payload);
	return 0;
}
