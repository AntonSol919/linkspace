.PHONY: all 

pages: index.html basics.html basics.old.html why.html lns.html about.html

%.html: %.md template.html
	pandoc	 -s ./$< --template ./template.html --metadata title=$@ -o $@

basics.html: basics.md template.html
	pandoc --toc -f markdown_strict+pipe_tables+header_attributes+bracketed_spans+implicit_header_references+backtick_code_blocks+footnotes+fancy_lists -t html5 ./$< --template ./template.html --metadata title="basics" -o $@
