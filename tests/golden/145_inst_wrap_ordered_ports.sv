module top;
  add4 u_add (a_in_signal, b_in_signal,
              sum_out_signal, carry_out_signal);
endmodule
// expected -----
module top;
  add4 u_add (
    a_in_signal,
    b_in_signal,
    sum_out_signal,
    carry_out_signal
  );
endmodule
