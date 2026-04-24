// Top banner
module a ( // start of ports
  input wire clk,   // clock
  /* reset signal */
  input wire rst_n,
  output logic q // result
);
  always_ff @(posedge clk) begin // sync
    q <= rst_n ? ~q : 1'b0; // toggle or clear
  end
endmodule // a
