#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if (size<3) return 0;
 
	leptSetStdNullHandler();

	PIX *pixs_payload = pixReadMemSpix(data, size);
	if (pixs_payload == NULL) return 0;

	PIX *pix_pointer_payload, *return_pix, *pix2;
	L_KERNEL *kel;
	NUMA *na1, *na2, *na3;

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixContrastTRCMasked(NULL, pix_pointer_payload, NULL, 0.5);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixDarkenGray(NULL, pix_pointer_payload, 220, 10);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixEqualizeTRC(NULL, pix_pointer_payload, 0.5, 10);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixGammaTRCMasked(NULL, pix_pointer_payload, NULL,
                                       1.0, 100, 175);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixGammaTRCWithAlpha(NULL, pix_pointer_payload,
                                          0.5, 1.0, 100);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixHalfEdgeByBandpass(pix_pointer_payload, 2, 2, 4, 4);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	l_float32 sat;
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	pixMeasureSaturation(pix_pointer_payload, 1, &sat);
	pixDestroy(&pix_pointer_payload);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixModifyBrightness(NULL, pix_pointer_payload, 0.5);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixModifyHue(NULL, pix_pointer_payload, 0.01 + 0.05 * 1);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixModifySaturation(NULL, pix_pointer_payload,
                                         -0.9 + 0.1 * 1);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixMosaicColorShiftRGB(pix_pointer_payload,
                                            -0.1, 0.0, 0.0, 0.0999, 1);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixMultConstantColor(pix_pointer_payload, 0.7, 0.4, 1.3);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);

	kel = kernelCreate(3, 3);
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	return_pix = pixMultMatrixColor( pix_pointer_payload, kel);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&return_pix);
	kernelDestroy(&kel);

	na1 = numaGammaTRC(1.0, 0, 255);
	na2 = numaGammaTRC(1.0, 0, 255);
	na3 = numaGammaTRC(1.0, 0, 255);
	pix_pointer_payload = pixCopy(NULL, pixs_payload);
	pix2 = pixMakeSymmetricMask(10, 10, 0.5, 0.5, L_USE_INNER);
	pixTRCMapGeneral(pix_pointer_payload, pix2, na1, na2, na3);
	numaDestroy(&na1);
	numaDestroy(&na2);
	numaDestroy(&na3);
	pixDestroy(&pix_pointer_payload);
	pixDestroy(&pix2);

	pixDestroy(&pixs_payload);
	return 0;
}
