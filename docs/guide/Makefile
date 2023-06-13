all: index.html

%.html: %.org
	emacsclient --eval "(progn (switch-to-buffer (find-file-noselect \"./$<\")) (org-html-export-to-html))"
