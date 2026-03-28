
if [ -z $1 ]; then
    echo "Usage: $0 <output_directory>"
    exit 1
fi

OUTPUT_DIRECTORY="$1"

for test_run_dir in $(ls ${OUTPUT_DIRECTORY}); do
    if [ -f "${OUTPUT_DIRECTORY}/${test_run_dir}/ptp4l_node2.log" ]; then
        echo "Processing ${OUTPUT_DIRECTORY}/${test_run_dir}/ptp4l_node2.log"
        ./ptp4l_graph.py "${OUTPUT_DIRECTORY}/${test_run_dir}/ptp4l_node2.log" --output "${OUTPUT_DIRECTORY}/${test_run_dir}/ptp4l_node2.png" --device "Node 2"
    fi
done