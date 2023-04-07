.PHONY: all 

index.html: index.md head tail
	pandoc --toc -f markdown_strict+header_attributes+bracketed_spans+implicit_header_references+backtick_code_blocks -t html5 index.md  | cat ./head - ./tail > $@
