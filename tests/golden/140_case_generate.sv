module m;
generate
case (W)
8: assign q = a;
16: assign q = b;
default: assign q = 0;
endcase
endgenerate
endmodule
// expected -----
module m;
  generate
    case (W)
      8: assign q = a;
      16: assign q = b;
      default: assign q = 0;
    endcase
  endgenerate
endmodule
