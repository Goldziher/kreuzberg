/*====================================================================*
 -  Copyright (C) 2001 Leptonica.  All rights reserved.
 -
 -  Redistribution and use in source and binary forms, with or without
 -  modification, are permitted provided that the following conditions
 -  are met:
 -  1. Redistributions of source code must retain the above copyright
 -     notice, this list of conditions and the following disclaimer.
 -  2. Redistributions in binary form must reproduce the above
 -     copyright notice, this list of conditions and the following
 -     disclaimer in the documentation and/or other materials
 -     provided with the distribution.
 -
 -  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 -  ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 -  LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
 -  A PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL ANY
 -  CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 -  EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 -  PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 -  PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY
 -  OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 -  NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 -  SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *====================================================================*/


/*
 * listtest.c
 *
 *    This file tests the main functions in the generic
 *    list facility, given in list.c and list.h.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include "pix_internal.h"

int main(int    argc,
         char **argv)
{
char    *filein;
l_int32  i, n, w, h, samecount, count;
BOX     *box, *boxc;
BOXA    *boxa, *boxan;
DLLIST  *head, *tail, *head2, *tail2, *elem, *nextelem;
PIX     *pixs;

    if (argc != 2)
        return ERROR_INT(" Syntax:  listtest filein", __func__, 1);
    filein = argv[1];
    setLeptDebugOK(1);

    boxa = boxan = NULL;
    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pix not made", __func__, 1);

        /* start with a boxa */
    boxa = pixConnComp(pixs, NULL, 4);
    n = boxaGetCount(boxa);

    /*-------------------------------------------------------*
     *        Do one of these two ...
     *-------------------------------------------------------*/
    if (1) {
            /* listAddToTail(): make a list by adding to tail */
        head = NULL;
        tail = NULL;
        for (i = 0; i < n; i++) {
            box = boxaGetBox(boxa, i, L_CLONE);
            listAddToTail(&head, &tail, box);
        }
    } else {
            /* listAddToHead(): make a list by adding to head */
        head = NULL;
        for (i = 0; i < n; i++) {
            box = boxaGetBox(boxa, i, L_CLONE);
            listAddToHead(&head, box);
        }
    }

        /* list concatenation */
    head2 = NULL;   /* cons up 2nd list from null */
    tail2 = NULL;
    for (i = 0; i < n; i++) {
        box = boxaGetBox(boxa, i, L_CLONE);
        listAddToTail(&head2, &tail2, box);
    }
    listJoin(&head, &head2);

    count = listGetCount(head);
    lept_stderr("%d items in list\n", count);
    listReverse(&head);
    count = listGetCount(head);
    lept_stderr("%d items in reversed list\n", count);
    listReverse(&head);
    count = listGetCount(head);
    lept_stderr("%d items in doubly reversed list\n", count);

    boxan = boxaCreate(n);

    /*-------------------------------------------------------*
     *        Then do one of these ...
     *-------------------------------------------------------*/
    if (1) {
            /* Removal of all elements and data from a list,
             * without using L_BEGIN_LIST_FORWARD macro */
        for (elem = head; elem; elem = nextelem) {
            nextelem = elem->next;
            box = (BOX *)elem->data;
            boxaAddBox(boxan, box, L_INSERT);
            elem->data = NULL;
            listRemoveElement(&head, elem);
        }
    } else if (0) {
            /* Removal of all elements and data from a list,
             * using L_BEGIN_LIST_FORWARD macro */
        L_BEGIN_LIST_FORWARD(head, elem)
            box = (BOX *)elem->data;
            boxaAddBox(boxan, box, L_INSERT);
            elem->data = NULL;
            listRemoveElement(&head, elem);
        L_END_LIST
    } else if (0) {
            /* Removal of all elements and data from a list,
             * using L_BEGIN_LIST_REVERSE macro */
        tail = listFindTail(head);
        L_BEGIN_LIST_REVERSE(tail, elem)
            box = (BOX *)elem->data;
            boxaAddBox(boxan, box, L_INSERT);
            elem->data = NULL;
            listRemoveElement(&head, elem);
        L_END_LIST
    } else if (0) {
            /* boxa and boxan are same when list made with listAddToHead() */
        tail = listFindTail(head);
        L_BEGIN_LIST_REVERSE(tail, elem)
            box = (BOX *)elem->data;
            boxaAddBox(boxan, box, L_INSERT);
            elem->data = NULL;
            listRemoveElement(&head, elem);
        L_END_LIST
        for (i = 0, samecount = 0; i < n; i++) {
            if (boxa->box[i]->w == boxan->box[i]->w  &&
                boxa->box[i]->h == boxan->box[i]->h)
                samecount++;
        }
        lept_stderr(" num boxes = %d, same count = %d\n",
                    boxaGetCount(boxa), samecount);
    } else if (0) {
            /* boxa and boxan are same when list made with listAddToTail() */
        L_BEGIN_LIST_FORWARD(head, elem)
            box = (BOX *)elem->data;
            boxaAddBox(boxan, box, L_INSERT);
            elem->data = NULL;
            listRemoveElement(&head, elem);
        L_END_LIST
        for (i = 0, samecount = 0; i < n; i++) {
            if (boxa->box[i]->w == boxan->box[i]->w  &&
                boxa->box[i]->h == boxan->box[i]->h)
                samecount++;
        }
        lept_stderr(" num boxes = %d, same count = %d\n",
                    boxaGetCount(boxa), samecount);
    } else if (0) {
            /* Destroy the boxes and then the list */
        L_BEGIN_LIST_FORWARD(head, elem)
            box = (BOX *)elem->data;
            boxDestroy(&box);
            elem->data = NULL;
        L_END_LIST
        listDestroy(&head);
    } else if (0) {
            /* listInsertBefore(): inserting a copy BEFORE each element */
        L_BEGIN_LIST_FORWARD(head, elem)
            box = (BOX *)elem->data;
            boxc = boxCopy(box);
            listInsertBefore(&head, elem, boxc);
        L_END_LIST
        L_BEGIN_LIST_FORWARD(head, elem)
            box = (BOX *)elem->data;
            boxaAddBox(boxan, box, L_INSERT);
            elem->data = NULL;
        L_END_LIST
        listDestroy(&head);
    } else if (0) {
            /* listInsertAfter(): inserting a copy AFTER that element */
        L_BEGIN_LIST_FORWARD(head, elem)
            box = (BOX *)elem->data;
            boxc = boxCopy(box);
            listInsertAfter(&head, elem, boxc);
        L_END_LIST
        L_BEGIN_LIST_FORWARD(head, elem)
            box = (BOX *)elem->data;
            boxaAddBox(boxan, box, L_INSERT);
            elem->data = NULL;
            listRemoveElement(&head, elem);
        L_END_LIST
/*        listDestroy(&head); */
    } else if (0) {
            /* Test listRemoveFromHead(), to successively
             * remove the head of the list for all elements. */
        count = 0;
        while (head) {
            box = (BOX *)listRemoveFromHead(&head);
            boxDestroy(&box);
            count++;
        }
        lept_stderr("removed %d items\n", count);
    } else if (0) {
            /* Another version to test listRemoveFromHead(), using
             * an iterator macro. */
        count = 0;
        L_BEGIN_LIST_FORWARD(head, elem)
            box = (BOX *)listRemoveFromHead(&head);
            boxDestroy(&box);
            count++;
        L_END_LIST
        lept_stderr("removed %d items\n", count);
    } else if (0) {
            /* test listRemoveFromTail(), to successively remove
             * the tail of the list for all elements. */
        count = 0;
        tail = NULL;   /* will find tail automatically */
        while (head) {
            box = (BOX *)listRemoveFromTail(&head, &tail);
            boxDestroy(&box);
            count++;
        }
        lept_stderr("removed %d items\n", count);
    } else if (0) {
            /* another version to test listRemoveFromTail(), using
             * an iterator macro. */
        count = 0;
        tail = listFindTail(head);  /* need to initialize tail */
        L_BEGIN_LIST_REVERSE(tail, elem)
            box = (BOX *)listRemoveFromTail(&head, &tail);
            boxDestroy(&box);
            count++;
        L_END_LIST
        lept_stderr("removed %d items\n", count);
    } else if (0) {
        /* Iterate backwards over the box array, and use
         * listFindElement() to find each corresponding data structure
         * within the list; then remove it.  Should completely
         * destroy the list.   Note that listFindElement()
         * returns the cell without removing it from the list! */
        n = boxaGetCount(boxa);
        for (i = 0, count = 0; i < n; i++) {
            box = boxaGetBox(boxa, n - i - 1, L_CLONE);
            if (i % 1709 == 0) boxPrintStreamInfo(stderr, box);
            elem = listFindElement(head, box);
            boxDestroy(&box);
            if (elem) {  /* found */
                box = (BOX *)listRemoveElement(&head, elem);
                if (i % 1709 == 0) boxPrintStreamInfo(stderr, box);
                boxDestroy(&box);
                count++;
            }
        }
        lept_stderr("removed %d items\n", count);
    }

    lept_stderr("boxa count = %d; boxan count = %d\n",
                boxaGetCount(boxa), boxaGetCount(boxan));
    boxaGetExtent(boxa, &w, &h, NULL);
    lept_stderr("boxa extent = (%d, %d)\n", w, h);
    boxaGetExtent(boxan, &w, &h, NULL);
    lept_stderr("boxan extent = (%d, %d)\n", w, h);

    pixDestroy(&pixs);
    boxaDestroy(&boxa);
    boxaDestroy(&boxan);
    return 0;
}

