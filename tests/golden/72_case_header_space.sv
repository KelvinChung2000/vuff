module m;
  logic a;
  initial begin
    case(x)
      1: a = 0;
      default: a = 1;
    endcase
  end
endmodule
// expected -----
module m;
  logic a;
  initial begin
    case (x)
      1: a = 0;
      default: a = 1;
    endcase
  end
endmodule
