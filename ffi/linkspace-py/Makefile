dev: setup_env
	. .env/bin/activate && ( cargo +nightly build ;  maturin develop )
	$(warn =============================================)
	$(warn   dont forget to `source .env/bin/activate`  )
	$(warn =============================================)

$(abspath .env/pyvenv.cfg):
	python3 -m venv .env
	pip3 install maturin

setup_env: $(abspath .env/pyvenv.cfg)

watch-dev: setup_env
	. .env/bin/activate && ( watchexec -r -e rs -- maturin develop )

doc: build
	. .env/bin/activate && pdoc linkspace

# this is a temporary hack https://pyo3.rs/v0.18.2/python_typing_hints
typestub: setup_env
	. .env/bin/activate && (pip install mypy ; stubgen -p linkspace)
	mv out/linkspace/linkspace.pyi ./
	cp ./linkspace.pyi ../../target/debug/
	rm -r out

build: setup_env
	. .env/bin/activate && (  maturin build )

install: build
	pip install --force-reinstall ../../target/wheels/linkspace-*.whl
