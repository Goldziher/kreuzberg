#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) { 
	if(size<3) return 0;
 
	leptSetStdNullHandler();

	BOXA *boxa_payload, *boxa1;
	boxa_payload = boxaReadMem(data, size);
	if(boxa_payload == NULL) return 0;

	PIX          *pixc, *pixd, *pix, *pixs;
	PIX          *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7;
	BOXAA        *baa;
	static const l_int32  WIDTH = 800;

	//boxaaDisplay()
	pix1 = pixRead("../test8.jpg");
	if(pix1!=NULL) {
		baa = boxaSort2d(boxa_payload, NULL, 6, 6, 5);
		pix2 = boxaaDisplay(pix1, baa, 3, 1, 0xff000000,
	                            0x00ff0000, 0, 0);
	  	boxaaDestroy(&baa);
		pixDestroy(&pix1);
		pixDestroy(&pix2);
	}
	
	//pixBlendBoxaRandom();
	pixc = pixRead("../test8.jpg");
	if(pixc!=NULL) {
		pixd = pixBlendBoxaRandom(pixc, boxa_payload, 0.4);
		pixDestroy(&pixc);
		pixDestroy(&pixd);
	}

	//pixDrawBoxa();
	pixc = pixRead("../test8.jpg");
	if(pixc!=NULL) {
		pixd = pixConvertTo1(pixc, 128);
		pix1 = pixConvertTo8(pixd, FALSE);
		pix2 = pixDrawBoxa(pix1, boxa_payload, 7, 0x40a0c000);
		pixDestroy(&pix1);
		pixDestroy(&pix2);
		pixDestroy(&pixc);
		pixDestroy(&pixd);
	}

	//pixMaskConnComp();
	pix1 = pixRead("../test8.jpg");
	if(pix1!=NULL) {
		boxa1 = boxaReadMem(data, size);
		if(boxa1==NULL) {
			pixDestroy(&pix1);
		}else{
			pix2 = pixScaleToSize(pix1, WIDTH, 0);
			pix3 = pixConvertTo1(pix2, 100);
			pix4 = pixExpandBinaryPower2(pix3, 2);
			pix5 = pixGenerateHalftoneMask(pix4, NULL, NULL, NULL);
			pix6 = pixMorphSequence(pix5, "c20.1 + c1.20", 0);
			pix7 = pixMaskConnComp(pix6, 8, &boxa1);
			boxaDestroy(&boxa1);
			pixDestroy(&pix1);
			pixDestroy(&pix2);
			pixDestroy(&pix3);
			pixDestroy(&pix4);
			pixDestroy(&pix5);
			pixDestroy(&pix6);
			pixDestroy(&pix7);
		}
	}

	//pixPaintBoxa();
	pix = pixRead("../test8.jpg");
	if(pix!=NULL) {
		boxa1 = boxaReadMem(data, size);
		if(boxa1==NULL) {
			pixDestroy(&pix);
		}else{
			pix1 = pixPaintBoxa(pix, boxa1, 0x60e0a000);
			pixDestroy(&pix);
			pixDestroy(&pix1);
			boxaDestroy(&boxa1);
		};
		
	}


	//pixPaintBoxaRandom();
	pix = pixRead("../test8.jpg");
	if(pix!=NULL) {
		boxa1 = boxaReadMem(data, size);
		if(boxa1==NULL) {
			pixDestroy(&pix);
		}else{
			pixs = pixConvertTo1(pix, 128);
			pix1 = pixPaintBoxaRandom(pixs, boxa1);    	
			pixDestroy(&pix);
			pixDestroy(&pixs);
			pixDestroy(&pix1);
			boxaDestroy(&boxa1);
		}
	}

	boxaDestroy(&boxa_payload);
	return 0;
}
