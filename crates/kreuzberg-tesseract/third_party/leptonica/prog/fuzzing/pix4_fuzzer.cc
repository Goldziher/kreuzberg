#include "leptfuzz.h"

extern "C" int
LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    leptSetStdNullHandler();

    PIX *pixs;
    BOX *box;

    pixs = pixReadMemSpix(data, size);
    if(pixs==NULL) return 0;

    PIX *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
    NUMA *na1, *na2, *na3, *na4, *na5, *na6;
    NUMAA *naa1;

    pix1 = pixConvertTo8(pixs, FALSE);
    box = boxCreate(120, 30, 200, 200);
    na1 = pixGetGrayHistogramInRect(pix1, box, 1);
    numaDestroy(&na1);
    boxDestroy(&box);
    pixDestroy(&pix1);

    naa1 = pixGetGrayHistogramTiled(pixs, 1, 1, 1);
    numaaDestroy(&naa1);

    pix1 = pixConvertTo8(pixs, FALSE);
    na1 = pixGetCmapHistogramMasked(pix1, NULL, 1, 1, 1);
    numaDestroy(&na1);
    pixDestroy(&pix1);

    pix1 = pixConvertTo8(pixs, FALSE);
    box = boxCreate(120, 30, 200, 200);
    na1 = pixGetCmapHistogramInRect(pix1, box, 1);
    numaDestroy(&na1);
    boxDestroy(&box);
    pixDestroy(&pix1);

    l_int32 ncolors;
    pixCountRGBColors(pixs, 1, &ncolors);

    l_uint32  pval;
    pix1 = pixConvertTo8(pixs, FALSE);
    pixGetPixelAverage(pix1, NULL, 10, 10, 1, &pval);
    pixDestroy(&pix1);

    pix1 = pixConvertTo8(pixs, FALSE);
    l_uint32  pval2;
    pixGetPixelStats(pix1, 1, L_STANDARD_DEVIATION, &pval2);
    pixDestroy(&pix1);

    pix1 = pixConvertTo8(pixs, FALSE);
    if(pix1!=NULL){
        pix2 = pixConvert8To32(pix1);
        pixGetAverageTiledRGB(pix2, 2, 2, L_MEAN_ABSVAL, &pix3, &pix4, &pix5);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
        pixDestroy(&pix5);
    }

    pixRowStats(pixs, NULL, &na1, &na2, &na3, &na4, &na5, &na6);
    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);
    numaDestroy(&na4);
    numaDestroy(&na5);
    numaDestroy(&na6);

    pixColumnStats(pixs, NULL, &na1, &na2, &na3, &na4, &na5, &na6);
    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);
    numaDestroy(&na4);
    numaDestroy(&na5);
    numaDestroy(&na6);

    static const l_int32  nbins = 10;
    l_int32     minval, maxval;
    l_uint32    *gau32;
    pix1 = pixScaleBySampling(pixs, 0.2, 0.2);
    pixGetBinnedComponentRange(pix1, nbins, 2, L_SELECT_GREEN,
                                   &minval, &maxval, &gau32, 0);
    pixDestroy(&pix1);
    lept_free(gau32);

    PIX *pixd = pixSeedspread(pixs, 4);
    PIX *pixc = pixConvertTo32(pixd);
    PIX *pixr = pixRankBinByStrip(pixc, L_SCAN_HORIZONTAL, 1,
		    		  10, L_SELECT_MAX);
    pixDestroy(&pixd);
    pixDestroy(&pixc);
    pixDestroy(&pixr);

    PIXA *pixa = pixaReadMem(data, size);
    pix1 = pixaGetAlignedStats(pixa, L_MEAN_ABSVAL, 2, 2);
    pixaDestroy(&pixa);
    pixDestroy(&pix1);

    l_int32 thresh, fgval, bgval;
    pix1 = pixConvertTo8(pixs, 0);
    pixSplitDistributionFgBg(pix1, 1.5, 1, &thresh, &fgval, &bgval, &pix2);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pixs);
    return 0;
}
