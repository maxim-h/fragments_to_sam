# Converter between fragments file and SAM record stream

This is a very simple and stupid program that takes in a [fragments file](https://support.10xgenomics.com/single-cell-atac/software/pipelines/latest/output/fragments), a list of chromosome sizes and outputs SAM to `stdout`.

It's written only for one purpose: to feed the output of [chromap](https://github.com/haowenz/chromap) to [Genrich](https://github.com/jsh58/Genrich), because the latter [doesn't support fragments files as input](https://github.com/jsh58/Genrich/issues/95), but can be fed SAM records from `stdin`.

Because of this hyperspecific purpose the code doesn't properly validate and form SAM records through noodles, but just hardcodes most of the record in hopes that it is correct.
This makes it fast enough not to slow Genrich down, but also dangerous for any other use. I don't really recommend it even for this use, but who am I to stop you ;)
