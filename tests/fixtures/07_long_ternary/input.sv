module a (input wire [31:0] x, output logic [31:0] y);
  assign y = (x == 32'h0) ? 32'hDEAD_BEEF : (x[0]) ? {x[30:0], 1'b1} : {x[30:0], 1'b0};
endmodule
