import sys
from huggingface_hub import snapshot_download
snapshot_download("Systran/faster-whisper-base", local_dir=sys.argv[1])
