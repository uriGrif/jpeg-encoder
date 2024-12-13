# JPEG Theory

This document is a small summary of what I've learned while working on this project.

This is just a basic summary to understand the gist of the JPEG compression process, so I recommend checking out the sources at the end of the document if you want to learn more about the subject.

<!-- TABLE OF CONTENTS -->
<details open>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#the-basic-idea">The Basic Idea</a>
    </li>
    <li>
      <a href="#color-space-conversion">Color Space Conversion</a>
    </li>
    <li>
      <a href="#chrominance-downsampling">Chrominance Downsampling</a>
    </li>
    <li>
      <a href="#quantization">Quantization</a>
    </li>
    <li>
      <a href="#run-length-and-huffman-encoding">Run Length and Huffman Encoding</a>
    </li>
    <li>
      <a href="#creating-the-file">Creating the File</a>
    </li>
    <li>
      <a href="#sources">Sources</a>
    </li>
  </ol>
</details>

## The Basic Idea

A digital image is made up by a collection of pixels, each one corresponding to a specific color. This colors are often represented as a combination of red, green and blue values (RGB), each one ranging from 0 to 255 (1 byte).

So, imagine an image of 1920x1080 pixels. If each pixel occupies 3 bytes, then the whole image would weight up to 6.2 MB. However, if you open any jpeg image file in your computer or phone, you will see that you have much bigger images that only weight a couple hundreds KB or even less. So, how can this be possible?

The JPEG algorithm rests on 2 facts about human vision:

-   Our eyes are bad at distinguishing subtle changes between colors, and are much better at distinguishing changes in brightness
-   Our vision generally discards high-frequency information. This basically means that you can easily see something like a straight separation between two colors (low-frequency information), but it's difficult to properly see details that have a lot of small changes in colors and shadows, like the leaves in a tree (high-frequency information)

These facts mean that we could remove a lot of information about an image without our eyes even noticing it. This is what the algorithm does, it removes "useless" information (hence it is called a "lossy" compression algorithm) and stores the remaining in a way the uses much less space than the original image.

The compression process can be divided into the following steps, each of which will be explained in detail below.

1. Color Space Conversion
2. Chrominance Downsampling
3. Discrete Cosine Transform
4. Quantization
5. Run Length and Huffman Encoding
6. Creating the File

## Color Space Conversion

A Color space is a specific organization of “channels” that represents colors. For example: RGB (a red, a green and a blue channel) or CMYK. All the pixels in an image have their individual color encoded in a certain color space.

The human eye is more sensitive to brightness than color, so the jpeg algorithm converts the images color space from RGB to a different color space that exploits this fact.

This new color space is denominated YCbCr which stands for Luminance (Y), Chrominance Blue (Cb) and Chrominance Red (Cr).

The Luminance channel contains the brightness information, while the chrominance channels contain the color information. This way, the algorithm can change the colors without affecting the brightness (which is the most important for human sight)

### RGB to YCbCr

There are multiple ways of converting from one to another, but I’m going to use [this one](https://en.wikipedia.org/wiki/YCbCr#JPEG_conversion)

```math
Y = 0 + 0.299*R + 0.587*G + 0.114*B \\
C_b = 128 - 0.168736*R - 0.331264*G + 0.5*B \\
C_r = 128 + 0.5*R - 0.418688*G - 0.081312*B
```

### About the BMP File

A quick comment on the .BMP image file, which I used as the input for my encoder. I chose this file format because it is the closest I found to a pure RGB matrix.

It stores the RGB values of an image, completely uncompressed, but in a particular order:

-   The pixels go left to right, bottom to top. This means that, the first pixel you read from a bmp file corresponds to the lower left corner of the image.
-   The pixel values are in fact stored in opposite order. This means that instead of RGB it would technically be BGR.
-   The amount of bytes for each row of the image must be divisible by 4. If it's not, the file stuffs the missing bytes with any value (usually 0).

## Chrominance Downsampling

For this step, we are going to separate each channel of the image, and only work with the Chrominance channels.

For blue and red chrominance channels, we are going to divide the image into blocks of 2x2 pixels. Then, we are going to take these 4 pixels, and merge them into a new pixel, whose value will be the average value of the 4 previous ones.

This way, our chrominance channels will now be only 1/4 of their original size. Of course, we are losing color information, but so far the luminance channel (which is the most important for human vision) remains intact.

If the image has an odd width or length, we must increase the odd dimension by one. Ideally, the new added pixels (we'll call this "padding" from now on), should have values that replicate the closest value to it (on the right or bottom edge of the image). In my case, however, I simply used 0 value for all paddings because it was easier.

### Subsampling Ratios:

_“The transformation into the [Y′CBCR color model](https://en.wikipedia.org/wiki/YCbCr) enables the next usual step, which is to reduce the spatial resolution of the Cb and Cr components (called "[downsampling](https://en.wikipedia.org/wiki/Downsampling)" or "[chroma subsampling](https://en.wikipedia.org/wiki/Chroma_subsampling)"). The ratios at which the downsampling is ordinarily done for JPEG images are [4:4:4](https://en.wikipedia.org/wiki/YUV_4:4:4) (no downsampling), [4:2:2](https://en.wikipedia.org/wiki/YUV_4:2:2) (reduction by a factor of 2 in the horizontal direction), or (most commonly) [4:2:0](https://en.wikipedia.org/wiki/YUV_4:2:0) (reduction by a factor of 2 in both the horizontal and vertical directions). For the rest of the compression process, Y', Cb and Cr are processed separately and in a very similar manner.” (source: [Wikipedia](https://en.wikipedia.org/wiki/JPEG#:~:text=The%20ratios%20at%20which%20the,the%20horizontal%20and%20vertical%20directions).)_

Important about subsampling formats: https://stackoverflow.com/questions/35497075/chroma-subsampling-algorithm-for-jpeg

## Discrete Cosine Transform

In the previous step, we accounted for the fact about colors and brightness. Now, this step is used for the second fact, about high-frequency information.

Actually understanding the Discrete Cosine Transform (DCT) involves a lot of complicated math, but the idea is the following:

We have this image, with 64 different patterns, each pattern has a size of 8x8 pixels (the same as the blocks in which we divided the image)

![](https://upload.wikimedia.org/wikipedia/commons/2/24/DCT-8x8.png)

Each square in this image represents different frequencies. We can se that the upper left squares correspond to lower frequencies, while the lower right correspond to higher frequencies.

This step will allow us to separate the important information from the disposable one (higher and lower frequency data), in the same way we did before with the color space conversion (brightness and colors).

Before performing the DCT, we need to do 2 important things:

-   For each channel, we divide the image into blocks of 8x8 pixels. From now on, all the steps will be based in 8x8 blocks (If the image size is not divisible by 8, we add padding to it, just like we did before)
-   We are also going to shift the range of values for each pixel. After the color space conversion step, each pixel has a value in the range [0 ; 255]. So, we are going to subtract 128 from each one, in order to change the range to [-128 ; 127]

The DCT basically consists in creating linear combinations of the patterns shown above. This means we add all the patterns together, each multiplied by a certain constant, and the sum represents a block.

The math tells us that we can reconstruct every block of the image with the right linear combination.

So, the aim of this step is to find the 64 constants (or coefficients) needed to make each block from the combination of the patterns.

After the process is done, for each 8x8 block of pixels in the image, for each color channel, we must have 64 coefficients. The first coefficient (the one constant that multiplies the upper left square, the white one) is called de DC coefficient (or constant components). The other 63 are called the AC coefficients (or alternating components). This will be important for later

### DCT calculation

The actual DCT formula looks like this:

![](https://i.gyazo.com/b9cec96387d83f263245f415c789f7cc.png)

As you can see, it involves the calculation of cosines and, generally speaking, our cpu using floating point arithmetic, which of course can be really slow.

There are approximations of this formula called BinDCT, which only use integers and byte shifting, resulting in a much faster calculation.

I used the one called "All-lifting binDCT-C" that appears in these papers:

-   https://citeseerx.ist.psu.edu/document?repid=rep1&type=pdf&doi=a16a78322dfdfc8c6ad1a38ba05caafe97a56254
-   https://thanglong.ece.jhu.edu/Tran/Pub/intDCT.pdf

I should say, however, that this algorithm didn't work out very well in my code. Although it is a bit faster, the jpeg images that result of using this approximation look bad (they have weird line patterns) and weight significantly more than the "real" DCT (but still much less than the original file).

As I've read, this approximation is used in most actual jpeg codec libraries and tools, so I am sure that this bad results are my fault. I probably must have messed up somewhere in the code.

## Quantization

As I explained above, the DCT separated the image information in terms of frequency. Now, it's time to "get rid" of the high frequency data.

We are gonna have a “Quantization table” with 64 values, each corresponding to the 64 constants calculated in the previous step. In this table, the upper left values are smaller, and the opposite are higher.

Next, we are going to divide each constant from the DCT by its corresponding value in the quantization table, and round each value to its closer integer.

Finally, in the resulting matrix, we will see that most of the values in the lower right side will be 0 (because the divisors are higher), thus eliminating unnecessary details.

### Considerations

-   The chrominance and luminance channels use different quantization tables. Because the luminance is more important for human eyesight, for the Y channel we use a “less aggressive” table, that ends up in less zeros (meaning less loss of detail).
-   In this step, we can specify the quality of the output image. A lower quality percentage will make the values in the quantization table bigger, therefore more divisions will end up being zero, and more details will be lost. You could have different Quantization tables for different quality values, such as 100%, 75%, 50%, etc. In my code, I only used one pair of tables.

## Run Length and Huffman Encoding

Finally, for each block of 8x8 pixels, we have a list of 64 integers that we can use to reconstruct the original block (using the same DCT patterns).

Next, we are going to list these numbers diagonally, from upper left to lower right. Like this:

![](https://upload.wikimedia.org/wikipedia/commons/thumb/4/43/JPEG_ZigZag.svg/220px-JPEG_ZigZag.svg.png)

If we see the list layed out this way, we will most likely see that the list ends up in a bunch of zeros. This is thanks to the quantization step.

### Run Length

The DC and AC coefficients are encoded in different ways. Let's start with the AC.

We will use a Run Length encoding algorithm (Entropy Coding), which writes a number, and then the amount of contiguous occurrences.

This way, instead of having, for example, a list like this:

[23, 45, 0, 0, 23, 23, 54, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]

We convert it into something smaller like this:

[23, 45, 0x2, 23x2, 54, 16, 0x10]

In the jpeg algorithm, however, we use a similar version, in which we don’t count the number of occurrences, but the number of zeros previous to each number. So, the list above would end up looking like this:

[23:0, 45:0, 23:2, 23:0, 54:0, 16:0, 0:9]

But this is not the way it actually looks.

For each non-zero value, we store 3 things:

-   Run Length: the amount of zeros previous to it.
-   Size: the amount of bits needed to represent the value. This allows us to only store the needed bits for the amplitude. For example: if the amplitude is 3, you don't need to use a whole byte, I now that only the next 2 bits correspond to the value.
-   Amplitude: the value itself (only necessary bits).

In the end, the list looks like this:

(RUNLENGHT; SIZE)(AMPLITUDE), (RUNLENGHT; SIZE)(AMPLITUDE), (RUNLENGHT; SIZE)(AMPLITUDE), ....

For the DC coefficients, instead of storing the value itself, for each block you store the value of the DC minus the value of the DC from the previous encoded block.

So, if you have 4 blocks with DC's: [100, 110, 130, 90], when you encode them, they will look like this:
[100, 10, 20, -40]

#### Considerations

-   The RUNLENGTH and the SIZE are stored together in one byte, each ocuppying 4 bits. This means that both values can only range from 0 to 15.
-   If you have more than 15 zeros (which you can't represent with only 4 bits) you use a "special" code (F;0) for each run of 15 zeros, skiping the amplitude.
-   Similarily, if there are no more non-zero values in the block, you have to use the End-Of-Block code (0;0) and skip the amplitude.
-   A coefficient's bit representation is not necessarily the same as its binary representation. If the value is negative, then you have to apply the following "conversion":
    ```rust
    if value < 0 { value + (1 << bit_length) - 1 } else { value }
    ```
    This way, a positive value and its negative both have the same minimum amount of bits.

### Huffman Encoding

Finally, we run everything through a Huffman coding algorithm to further compress the data.
To fully understand how Huffman coding works, I recommend consulting other sources (there are lots of great resources to understand it). The basic idea is this:

If you need to represent, for example, 200 different values (they may be ASCII characters, or, in this case, (RUNLENGHT;SIZE) pairs), you would have to use 8 bits for every value. However, most of the time, there are some values that are way more frequent that others. Huffman coding sugests that we assign these values codes that are shorter than 8 bits, and the less frequent values can have even longer codes.

This way, for most of the values we are gonna use less bits than would actually be necessary, thus being able to compress the data even more.

The Huffman algorithm uses a table to pair the values to encode to their corresponding variable length bit codes.

Although this tables and codes could be generated for every specific batch of data (you calculate the frequency for each value and then assign codes based on that), the JPEG standard provides some general use tables, with a common way of getting the codes. This is best explained in the video playlist I left on the sources, I recommend watching it.

#### About the blocks

This encoding process is performed in blocks, for each color channel. There are different ways of storing this information in the scan of an image, but I used one called Interleaved. This means that, for each section of the image, we store the Y, the Cb and the Cr coded blocks consecutively, so the data looks like this: Y,Cb,Cr,Y,Cb,Cr,Y,Cb,Cr,Y,Cb,Cr...

But wait, what if we downsampled the chrominance channels? These have less pixels than the Luminance channels, and therefore less 8x8 blocks. Well, suppose we used a 4:2:0 ratio, then for each dimension, the Y channel has double the pixels than de Cb and Cr, so, for each chrominance block, there are 4 luminance blocks.

In the end, the data would look like this: Y,Y,Y,Y,Cb,Cr,Y,Y,Y,Y,Cb,Cr,Y,Y,Y,Y,Cb,Cr,Y,Y,Y,Y,Cb,Cr...

## Creating the File

Now comes the last part, which is generating the proper JPEG file. I think it's better to just leave this image, which shows how a jpeg file looks in hex mode.

We can see that a file has different headers, containing things like the file version, image metadata like the width and height, the necessary structures to decode the data (quantization and huffman tables), and the data itself.

![](https://raw.githubusercontent.com/corkami/formats/master/image/JPEGRGB_dissected.png)

Check the sources to learn in detail about each header.

## Sources

I have to give a special place to this playlist/course done by Daniel Harding, "Everything you need to know about JPEG". The videos are great and they explain everything clearly and in detail. Plus, there is C++ code to see the implementation (both encoding and decoding). This was my primary source for this project.

-   [Youtube playlist](https://www.youtube.com/playlist?list=PLpsTn9TA_Q8VMDyOPrDKmSJYt1DLgDZU4)
-   [Github code](https://github.com/dannye/jed)

General information about JPEG:

-   [Introduction video to JPEG](https://www.youtube.com/watch?v=Kv1Hiv3ox8I)
-   [JPEG Wikipedia article](https://en.wikipedia.org/wiki/JPEG)
-   [Let's Write a Simple JPEG Library, Part-I: The Basics](https://koushtav.me/jpeg/tutorial/2017/11/25/lets-write-a-simple-jpeg-library-part-1/)
-   [JPEG guide](https://www.thewebmaster.com/jpeg-definitive-guide/)
-   [Presentation on JPEG](https://www.slideshare.net/slideshow/jpeg-73701342/73701342#1)

JPEG File Structure:

-   [JPEG file format and headers](https://github.com/corkami/formats/blob/master/image/jpeg.md)
-   [Wiki on headers](https://en.wikibooks.org/wiki/JPEG_-_Idea_and_Practice/The_header_part)
-   [JPEG header viewer](https://cyber.meme.tips/jpdump/)

DCT:

-   [Computerphile Video](https://www.youtube.com/watch?v=Q2aEzeMDHMA)
-   [DCT explained with matlab](https://www.youtube.com/watch?v=mUKPy3r0TTI)
-   [BinDCT paper](https://thanglong.ece.jhu.edu/Tran/Pub/intDCT.pdf)

Huffman Coding:

-   [About Huffman Coding](https://www.youtube.com/watch?v=JsTptu56GM8)

Color Spaces:

-   [RGB to YCbCr conversion](https://sistenix.com/rgb2ycbcr.html)
