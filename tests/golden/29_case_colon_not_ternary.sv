module m (input wire [1:0] sel, output logic q);
  always_comb begin
    case (sel)
      2'd0: q = 1'b0;
      2'd1: q = 1'b1;
      default: q = 1'bx;
    endcase
  end
endmodule
// expected -----
module m (
  input  wire  [1:0] sel,
  output logic       q
);
  always_comb begin
    case (sel)
      2'd0: q = 1'b0;
      2'd1: q = 1'b1;
      default: q = 1'bx;
    endcase
  end
endmodule
