FROM gcr.io/oss-fuzz-base/base-builder-rust
RUN git clone --depth 1 https://github.com/WyattAu/crawlkit.git crawlkit
WORKDIR crawlkit
COPY build.sh $SRC/
