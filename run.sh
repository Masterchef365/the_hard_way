rm -f /tmp/monado_comp_ipc
if cargo build; then
    sh -c 'sleep 0.2 && cargo run; killall monado-service' &
    monado-service
fi 
