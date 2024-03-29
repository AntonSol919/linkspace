#!/bin/env python3
import os
from linkspace import *
import sys
if len(sys.argv) < 5:
    sys.exit('Usage: imagefile boardname x y')
[imagefile,boardname,x,y] = sys.argv[1:]
x = int(x)
y = int(y)
if x > 1000 or y > 1000:
    sys.exit('X and Y coordinates should be < 1000')



imgdata = open(imagefile,'rb').read()
# this will error if the file is to large ( 2^16 - 512 ). 
# To use bigger files you can manually split/merge them, or wait for a convention to stabilize 
datap = lk_datapoint(imgdata)
# we can access the point's fields as bytes such as datap.hash, turn those into b64
# print("Saving image ",base64(datap.hash))
# Alternatively we can use lk_eval/lk_eval2str and use an abe expr
print(lk_eval2str("Using image [hash:str]",datap))


# We make up this scheme for our app
# Tags will be decimal encoded, ptr will point to image data. 
tag = f"{x:08d}{y:08d}".encode() # Everything in linkspace is plain bytes
links = [Link(tag,datap.hash)]
linkp = lk_linkpoint(domain=b"imageboard",
                     path=[boardname.encode()],
                     links=links)
# print(lk_eval2str("Placing new image [pkt]",linkp))

# instance looks for 'path' arg | $LK_DIR env | $HOME/linkspace
lk = lk_open(create=True) 

# write the point to the linkspace instance
_isnew = lk_save(lk,datap)
lk_save(lk,linkp)
