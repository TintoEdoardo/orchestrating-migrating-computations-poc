# Partially identical to build.

# Remove existing `requests` folder.
rm -r out/requests &> /dev/null

# Copy the original `requests` folder in `out/`.
cp -r requests out/requests