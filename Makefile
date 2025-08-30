.PHONY: serve
serve:
	python3 -m http.server --directory test-site/ 5000
