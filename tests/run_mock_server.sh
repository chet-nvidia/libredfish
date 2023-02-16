#!/bin/bash

pip install --requirement requirements.txt

python redfishMockupServer.py --port 1266 --dir mockups/dell/ --ssl --cert cert.pem --key key.pem

